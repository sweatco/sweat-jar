use std::cmp;

use near_sdk::{env, ext_contract, is_promise_success, json_types::U128, near_bindgen, AccountId, PromiseOrValue};
use sweat_jar_model::{
    api::ClaimApi,
    claimed_amount_view::ClaimedAmountView,
    jar::{AggregatedTokenAmountView, JarIdView},
    U32,
};

use crate::{
    event::{emit, ClaimEventItem, EventKind},
    jar::model::{ClaimData, ClaimJar, Jar},
    Contract, ContractExt, JarsStorage,
};

#[ext_contract(ext_self)]
pub trait ClaimCallbacks {
    fn after_claim(
        &mut self,
        claimed_amount: ClaimedAmountView,
        claim_data: ClaimData,
        event: EventKind,
    ) -> ClaimedAmountView;
}

#[near_bindgen]
impl ClaimApi for Contract {
    fn claim_total(&mut self, detailed: Option<bool>) -> PromiseOrValue<ClaimedAmountView> {
        let account_id = env::predecessor_account_id();
        self.migrate_account_jars_if_needed(account_id.clone());
        let jar_ids = self.account_jars(&account_id).iter().map(|a| U32(a.id)).collect();
        self.claim_jars_internal(account_id, jar_ids, None, detailed)
    }

    fn claim_jars(
        &mut self,
        jar_ids: Vec<JarIdView>,
        amount: Option<U128>,
        detailed: Option<bool>,
    ) -> PromiseOrValue<ClaimedAmountView> {
        let account_id = env::predecessor_account_id();
        self.migrate_account_jars_if_needed(account_id.clone());
        self.claim_jars_internal(account_id, jar_ids, amount, detailed)
    }
}

impl Contract {
    fn claim_jars_internal(
        &mut self,
        account_id: AccountId,
        jar_ids: Vec<JarIdView>,
        amount: Option<U128>,
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

        let jars = unlocked_jars
            .into_iter()
            .map(|jar| {
                let product = self.get_product(&jar.product_id);
                let (available_interest, remainder) = jar.get_interest(&product, now);

                let interest_to_claim = amount.map_or(available_interest, |amount| {
                    cmp::min(available_interest, amount.0 - accumulator.get_total().0)
                });

                if interest_to_claim > 0 {
                    let jar = self.get_jar_mut_internal(&jar.account_id, jar.id);

                    jar.claim_remainder = remainder;

                    jar.lock();

                    accumulator.add(jar.id, interest_to_claim);

                    event_data.push(ClaimEventItem {
                        id: jar.id,
                        interest_to_claim: U128(interest_to_claim),
                    });
                }

                ClaimJar {
                    jar_id: jar.id,
                    available_yield: available_interest,
                    claimed_amount: interest_to_claim,
                }
            })
            .collect();

        if accumulator.get_total().0 > 0 {
            self.claim_interest(
                accumulator,
                ClaimData { account_id, now, jars },
                EventKind::Claim(event_data),
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
        claimed_amount: ClaimedAmountView,
        claim_data: ClaimData,
        event: EventKind,
    ) -> PromiseOrValue<ClaimedAmountView> {
        PromiseOrValue::Value(self.after_claim_internal(
            claimed_amount,
            claim_data,
            event,
            crate::common::test_data::get_test_future_success(),
        ))
    }

    #[cfg(not(test))]
    #[mutants::skip] // Covered by integration tests
    fn claim_interest(
        &mut self,
        claimed_amount: ClaimedAmountView,
        claim_data: ClaimData,
        event: EventKind,
    ) -> PromiseOrValue<ClaimedAmountView> {
        use crate::{
            common::gas_data::{GAS_FOR_AFTER_CLAIM, GAS_FOR_FT_TRANSFER},
            ft_interface::FungibleTokenInterface,
            internal::assert_gas,
        };

        assert_gas(GAS_FOR_FT_TRANSFER.as_gas() * 2 + GAS_FOR_AFTER_CLAIM.as_gas(), || {
            format!("claim_interest: number of jars: {}", claim_data.jars.len())
        });

        self.ft_contract()
            .ft_transfer(&claim_data.account_id, claimed_amount.get_total().0, "claim", &None)
            .then(after_claim_call(claimed_amount, claim_data, event))
            .into()
    }

    fn after_claim_internal(
        &mut self,
        claimed_amount: ClaimedAmountView,
        claim_data: ClaimData,
        event: EventKind,
        is_promise_success: bool,
    ) -> ClaimedAmountView {
        if !is_promise_success {
            for claim in claim_data.jars {
                let jar_id = claim.jar_id;

                self.get_jar_mut_internal(&claim_data.account_id, jar_id).unlock();
            }

            return match claimed_amount {
                ClaimedAmountView::Total(_) => ClaimedAmountView::Total(U128(0)),
                ClaimedAmountView::Detailed(_) => ClaimedAmountView::Detailed(AggregatedTokenAmountView::default()),
            };
        }

        for claim_jar in claim_data.jars {
            let jar = self
                .account_jars
                .get_mut(&claim_data.account_id)
                .unwrap_or_else(|| env::panic_str(&format!("Account '{}' doesn't exist", claim_data.account_id)))
                .get_jar_mut(claim_jar.jar_id);

            let product = self
                .products
                .get(&jar.product_id)
                .unwrap_or_else(|| env::panic_str(&format!("Product '{}' doesn't exist", jar.product_id)));

            jar.claim(claim_jar.available_yield, claim_jar.claimed_amount, claim_data.now)
                .unlock();

            if jar.should_be_closed(&product, claim_data.now) {
                self.delete_jar(&claim_data.account_id, claim_jar.jar_id);
            }
        }

        emit(event);

        claimed_amount
    }
}

#[near_bindgen]
impl ClaimCallbacks for Contract {
    #[private]
    fn after_claim(
        &mut self,
        claimed_amount: ClaimedAmountView,
        claim_data: ClaimData,
        event: EventKind,
    ) -> ClaimedAmountView {
        self.after_claim_internal(claimed_amount, claim_data, event, is_promise_success())
    }
}

#[cfg(not(test))]
#[mutants::skip] // Covered by integration tests
fn after_claim_call(claimed_amount: ClaimedAmountView, claim_data: ClaimData, event: EventKind) -> near_sdk::Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(crate::common::gas_data::GAS_FOR_AFTER_CLAIM)
        .after_claim(claimed_amount, claim_data, event)
}
