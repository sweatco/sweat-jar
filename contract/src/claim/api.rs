use std::collections::HashMap;

use near_sdk::{env, ext_contract, json_types::U128, near_bindgen, AccountId, PromiseOrValue};
use sweat_jar_model::{
    api::ClaimApi, claimed_amount_view::ClaimedAmountView, jar::AggregatedTokenAmountView, ProductId, TokenAmount,
};

use crate::{
    event::{emit, EventKind},
    internal::is_promise_success,
    jar::{
        account::v1::AccountV1Companion,
        model::{Jar, JarCompanion},
    },
    product::model::v1::InterestCalculator,
    Contract, ContractExt,
};

#[allow(dead_code)] // False positive since rust 1.78. It is used from `ext_contract` macro.
#[ext_contract(ext_self)]
pub trait ClaimCallbacks {
    fn after_claim(
        &mut self,
        account_id: AccountId,
        claimed_amount: ClaimedAmountView,
        account_rollback: AccountV1Companion,
        event: EventKind,
    ) -> ClaimedAmountView;
}

#[near_bindgen]
impl ClaimApi for Contract {
    fn claim_total(&mut self, detailed: Option<bool>) -> PromiseOrValue<ClaimedAmountView> {
        let account_id = env::predecessor_account_id();
        self.assert_migrated(&account_id);

        let account = self.get_account(&account_id);
        let mut accumulator = ClaimedAmountView::new(detailed);
        let now = env::block_timestamp_ms();

        let mut rollback_jars = HashMap::new();
        let mut interest_per_jar: HashMap<ProductId, (TokenAmount, u64)> = HashMap::new();

        env::log_str(format!("Account {:?} has {} jars", account_id.clone(), account.jars.len()).as_str());

        for (product_id, jar) in &account.jars {
            if jar.is_pending_withdraw {
                continue;
            }

            rollback_jars.insert(product_id.clone(), jar.to_rollback());

            let product = self.get_product(product_id);
            let (interest, remainder) = product.terms.get_interest(account, jar, now);

            env::log_str(format!("For jar {} claim {}", product_id.clone(), interest).as_str());

            if interest == 0 {
                continue;
            }

            interest_per_jar.insert(product_id.clone(), (interest, remainder));
            accumulator.add(product_id, interest);
        }

        let account = self.get_account_mut(&account_id);
        for (product_id, (interest, remainder)) in interest_per_jar {
            let jar = account.get_jar_mut(&product_id);
            jar.claim(interest, remainder, now).lock();
        }

        let account_rollback = AccountV1Companion {
            score: Some(account.score),
            jars: Some(rollback_jars),
            ..AccountV1Companion::default()
        };

        account.score.try_reset_score();

        if accumulator.get_total().0 > 0 {
            self.claim_interest(
                &account_id,
                accumulator,
                account_rollback,
                // TODO: add events
                EventKind::Claim(vec![]),
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
        account_id: &AccountId,
        claimed_amount: ClaimedAmountView,
        account_rollback: AccountV1Companion,
        event: EventKind,
    ) -> PromiseOrValue<ClaimedAmountView> {
        PromiseOrValue::Value(self.after_claim_internal(
            account_id.clone(),
            claimed_amount,
            account_rollback,
            event,
            is_promise_success(),
        ))
    }

    #[cfg(not(test))]
    #[mutants::skip] // Covered by integration tests
    fn claim_interest(
        &mut self,
        account_id: &AccountId,
        claimed_amount: ClaimedAmountView,
        account_rollback: AccountV1Companion,
        event: EventKind,
    ) -> PromiseOrValue<ClaimedAmountView> {
        use crate::{
            common::gas_data::{GAS_FOR_AFTER_CLAIM, GAS_FOR_FT_TRANSFER},
            ft_interface::FungibleTokenInterface,
            internal::assert_gas,
        };

        assert_gas(GAS_FOR_FT_TRANSFER.as_gas() * 2 + GAS_FOR_AFTER_CLAIM.as_gas(), || {
            "Not enough gas for claim".to_string()
        });

        self.ft_contract()
            .ft_transfer(account_id, claimed_amount.get_total().0, "claim", &None)
            .then(after_claim_call(
                account_id.clone(),
                claimed_amount,
                account_rollback,
                event,
            ))
            .into()
    }

    fn after_claim_internal(
        &mut self,
        account_id: AccountId,
        claimed_amount: ClaimedAmountView,
        account_rollback: AccountV1Companion,
        event: EventKind,
        is_promise_success: bool,
    ) -> ClaimedAmountView {
        if is_promise_success {
            let account = self.accounts.get_mut(&account_id).expect("Account is not found");
            let jars = account_rollback.jars.expect("Jars are required in rollback account");

            for (product_id, _) in jars {
                let jar = account.get_jar_mut(&product_id);
                jar.unlock();

                if jar.should_close() {
                    account.jars.remove(&product_id);
                }
            }

            emit(event);

            claimed_amount
        } else {
            let account = self.get_account_mut(&account_id);
            account.apply(&account_rollback);

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
        account_id: AccountId,
        claimed_amount: ClaimedAmountView,
        account_rollback: AccountV1Companion,
        event: EventKind,
    ) -> ClaimedAmountView {
        self.after_claim_internal(
            account_id,
            claimed_amount,
            account_rollback,
            event,
            is_promise_success(),
        )
    }
}

#[cfg(not(test))]
#[mutants::skip] // Covered by integration tests
fn after_claim_call(
    account_id: AccountId,
    claimed_amount: ClaimedAmountView,
    account_rollback: AccountV1Companion,
    event: EventKind,
) -> near_sdk::Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(crate::common::gas_data::GAS_FOR_AFTER_CLAIM)
        .after_claim(account_id, claimed_amount, account_rollback, event)
}

impl Jar {
    fn to_rollback(&self) -> JarCompanion {
        JarCompanion {
            is_pending_withdraw: Some(false),
            claimed_balance: Some(self.claimed_balance),
            claim_remainder: Some(self.claim_remainder),
            cache: Some(self.cache),
            ..JarCompanion::default()
        }
    }
}
