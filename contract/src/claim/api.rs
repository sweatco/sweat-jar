use near_sdk::{env, ext_contract, json_types::U128, near_bindgen, AccountId, PromiseOrValue};
use sweat_jar_model::{
    api::ClaimApi,
    claimed_amount_view::ClaimedAmountView,
    jar::{AggregatedTokenAmountView, JarIdView},
    U32,
};

use crate::{
    common::Timestamp,
    event::{emit, ClaimEventItem, EventKind},
    internal::is_promise_success,
    jar::model::Jar,
    score::AccountScore,
    Contract, ContractExt, JarsStorage,
};

#[allow(dead_code)] // False positive since rust 1.78. It is used from `ext_contract` macro.
#[ext_contract(ext_self)]
pub trait ClaimCallbacks {
    fn after_claim(
        &mut self,
        claimed_amount: ClaimedAmountView,
        jars_before_transfer: Vec<Jar>,
        score_before_transfer: Option<AccountScore>,
        event: EventKind,
        now: Timestamp,
    ) -> ClaimedAmountView;
}

#[near_bindgen]
impl ClaimApi for Contract {
    fn claim_total(&mut self, detailed: Option<bool>) -> PromiseOrValue<ClaimedAmountView> {
        let account_id = env::predecessor_account_id();
        self.migrate_account_if_needed(&account_id);
        let jar_ids = self.account_jars(&account_id).iter().map(|a| U32(a.id)).collect();
        self.claim_jars_internal(account_id, jar_ids, detailed)
    }
}

impl Contract {
    fn claim_jars_internal(
        &mut self,
        account_id: AccountId,
        jar_ids: Vec<JarIdView>,
        detailed: Option<bool>,
    ) -> PromiseOrValue<ClaimedAmountView> {
        let now = env::block_timestamp_ms();
        let mut accumulator = ClaimedAmountView::new(detailed);

        let unlocked_jars: Vec<Jar> = self
            .account_jars(&account_id)
            .iter()
            .filter(|jar| !jar.is_pending_withdraw && jar_ids.contains(&U32(jar.id)))
            .cloned()
            .collect();

        let mut event_data: Vec<ClaimEventItem> = vec![];

        let account_score = self.get_score_mut(&account_id);

        let account_score_before_transfer = account_score.as_ref().map(|s| **s);

        let score = account_score.map(AccountScore::claim_score).unwrap_or_default();

        for jar in &unlocked_jars {
            let product = self.get_product(&jar.product_id);
            let (interest, remainder) = jar.get_interest(&score, &product, now);

            if interest > 0 {
                let jar = self.get_jar_mut_internal(&jar.account_id, jar.id);

                jar.claim_remainder = remainder;

                jar.claim(interest, now).lock();

                accumulator.add(jar.id, interest);

                event_data.push((jar.id, U128(interest)));
            }
        }

        if accumulator.get_total().0 > 0 {
            self.claim_interest(
                &account_id,
                accumulator,
                unlocked_jars,
                account_score_before_transfer,
                EventKind::Claim(event_data),
                now,
            )
        } else {
            PromiseOrValue::Value(accumulator)
        }
    }
}

impl Contract {
    #[cfg(test)]
    fn claim_interest(
        &mut self,
        _account_id: &AccountId,
        claimed_amount: ClaimedAmountView,
        jars_before_transfer: Vec<Jar>,
        score_before_transfer: Option<AccountScore>,
        event: EventKind,
        now: Timestamp,
    ) -> PromiseOrValue<ClaimedAmountView> {
        PromiseOrValue::Value(self.after_claim_internal(
            claimed_amount,
            jars_before_transfer,
            score_before_transfer,
            event,
            now,
            is_promise_success(),
        ))
    }

    #[cfg(not(test))]
    #[mutants::skip] // Covered by integration tests
    fn claim_interest(
        &mut self,
        account_id: &AccountId,
        claimed_amount: ClaimedAmountView,
        jars_before_transfer: Vec<Jar>,
        score_before_transfer: Option<AccountScore>,
        event: EventKind,
        now: Timestamp,
    ) -> PromiseOrValue<ClaimedAmountView> {
        use crate::{
            common::gas_data::{GAS_FOR_AFTER_CLAIM, GAS_FOR_FT_TRANSFER},
            ft_interface::FungibleTokenInterface,
            internal::assert_gas,
        };

        assert_gas(GAS_FOR_FT_TRANSFER.as_gas() * 2 + GAS_FOR_AFTER_CLAIM.as_gas(), || {
            format!("claim_interest: number of jars: {}", jars_before_transfer.len())
        });

        self.ft_contract()
            .ft_transfer(account_id, claimed_amount.get_total().0, "claim", &None)
            .then(after_claim_call(
                claimed_amount,
                jars_before_transfer,
                score_before_transfer,
                event,
                now,
            ))
            .into()
    }

    fn after_claim_internal(
        &mut self,
        claimed_amount: ClaimedAmountView,
        jars_before_transfer: Vec<Jar>,
        score_before_transfer: Option<AccountScore>,
        event: EventKind,
        now: Timestamp,
        is_promise_success: bool,
    ) -> ClaimedAmountView {
        if is_promise_success {
            for jar_before_transfer in jars_before_transfer {
                let product = self.products.get(&jar_before_transfer.product_id).unwrap_or_else(|| {
                    env::panic_str(&format!("Product '{}' doesn't exist", jar_before_transfer.product_id))
                });

                let score = self
                    .get_score(&jar_before_transfer.account_id)
                    .map(AccountScore::claimable_score)
                    .unwrap_or_default();

                let jar = self
                    .accounts
                    .get_mut(&jar_before_transfer.account_id)
                    .unwrap_or_else(|| {
                        env::panic_str(&format!("Account '{}' doesn't exist", jar_before_transfer.account_id))
                    })
                    .get_jar_mut(jar_before_transfer.id);

                jar.unlock();

                if jar.should_be_closed(&score, &product, now) {
                    self.delete_jar(&jar_before_transfer.account_id, jar_before_transfer.id);
                }
            }

            emit(event);

            claimed_amount
        } else {
            let account_id = jars_before_transfer
                .first()
                .expect("After claim without jars")
                .account_id
                .clone();

            for jar_before_transfer in jars_before_transfer {
                let jar_id = jar_before_transfer.id;
                *self.get_jar_mut_internal(&account_id, jar_id) = jar_before_transfer.unlocked();
            }

            if let Some(score) = score_before_transfer {
                self.accounts
                    .get_mut(&account_id)
                    .unwrap_or_else(|| panic!("Account: {account_id} does not exist"))
                    .score = score;
            }

            match claimed_amount {
                ClaimedAmountView::Total(_) => ClaimedAmountView::Total(U128(0)),
                ClaimedAmountView::Detailed(_) => ClaimedAmountView::Detailed(AggregatedTokenAmountView::default()),
            }
        }
    }
}

#[near_bindgen]
impl ClaimCallbacks for Contract {
    #[private]
    fn after_claim(
        &mut self,
        claimed_amount: ClaimedAmountView,
        jars_before_transfer: Vec<Jar>,
        score_before_transfer: Option<AccountScore>,
        event: EventKind,
        now: Timestamp,
    ) -> ClaimedAmountView {
        self.after_claim_internal(
            claimed_amount,
            jars_before_transfer,
            score_before_transfer,
            event,
            now,
            is_promise_success(),
        )
    }
}

#[cfg(not(test))]
#[mutants::skip] // Covered by integration tests
fn after_claim_call(
    claimed_amount: ClaimedAmountView,
    jars_before_transfer: Vec<Jar>,
    score_before_transfer: Option<AccountScore>,
    event: EventKind,
    now: Timestamp,
) -> near_sdk::Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(crate::common::gas_data::GAS_FOR_AFTER_CLAIM)
        .after_claim(claimed_amount, jars_before_transfer, score_before_transfer, event, now)
}
