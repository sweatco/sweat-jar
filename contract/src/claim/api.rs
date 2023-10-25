use std::cmp;

use model::{jar::JarIdView, AggregatedTokenAmountView, U32};
use near_sdk::{env, ext_contract, is_promise_success, json_types::U128, near_bindgen, AccountId, PromiseOrValue};

use crate::{
    claim::view::ClaimedAmountView,
    event::{emit, ClaimEventItem, EventKind},
    jar::model::Jar,
    Contract, ContractExt,
};

/// The `ClaimApi` trait defines methods for claiming interest from jars within the smart contract.
pub trait ClaimApi {
    /// Claims all available interest from all deposit jars belonging to the calling account.
    ///
    /// * `detailed` – An optional boolean value specifying if the method must return only total amount of claimed tokens
    ///                or detailed summary for each claimed jar. Set it `true` to get a detailed result. In case of `false`
    ///                or `None` it returns only the total claimed amount.
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<ClaimedAmountView>` representing the amount of tokens claimed
    /// and probably a map containing amount of tokens claimed from each Jar. If the total available
    /// interest across all jars is zero, the returned value will also be zero and the detailed map will be empty (if requested).
    fn claim_total(&mut self, detailed: Option<bool>) -> PromiseOrValue<ClaimedAmountView>;

    /// Claims interest from specific deposit jars with provided IDs.
    ///
    /// # Arguments
    ///
    /// * `jar_ids` - A `Vec<JarId>` containing the IDs of the deposit jars from which interest is being claimed.
    /// * `amount` - An optional `TokenAmount` specifying the desired amount of tokens to claim. If provided, the method
    ///              will attempt to claim this specific amount of tokens. If not provided or if the specified amount
    ///              is greater than the total available interest in the provided jars, the method will claim the maximum
    ///              available amount.
    /// * `detailed` – An optional boolean value specifying if the method must return only total amount of claimed tokens
    ///                or detailed summary for each claimed jar. Set it `true` to get a detailed result. In case of `false`
    ///                or `None` it returns only the total claimed amount.  
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<ClaimedAmountView>` representing the total amount of tokens claimed
    /// and probably a map containing amount of tokens claimed from each Jar.
    /// If the total available interest across the specified jars is zero or the provided `amount`
    /// is zero, the total amount in returned object will also be zero and the detailed map will be empty (if requested).
    fn claim_jars(
        &mut self,
        jar_ids: Vec<JarIdView>,
        amount: Option<U128>,
        detailed: Option<bool>,
    ) -> PromiseOrValue<ClaimedAmountView>;
}

#[ext_contract(ext_self)]
pub trait ClaimCallbacks {
    fn after_claim(
        &mut self,
        claimed_amount: ClaimedAmountView,
        jars_before_transfer: Vec<Jar>,
        event: EventKind,
    ) -> ClaimedAmountView;
}

#[near_bindgen]
impl ClaimApi for Contract {
    fn claim_total(&mut self, detailed: Option<bool>) -> PromiseOrValue<ClaimedAmountView> {
        let account_id = env::predecessor_account_id();
        let jar_ids = self.account_jars(&account_id).iter().map(|a| U32(a.id)).collect();

        let accumulator = &mut if detailed.unwrap_or(false) {
            ClaimedAmountView::Detailed(AggregatedTokenAmountView::default())
        } else {
            ClaimedAmountView::Total(U128(0))
        };

        self.claim_jars_internal(jar_ids, None, accumulator)
    }

    fn claim_jars(
        &mut self,
        jar_ids: Vec<JarIdView>,
        amount: Option<U128>,
        detailed: Option<bool>,
    ) -> PromiseOrValue<ClaimedAmountView> {
        let accumulator = &mut if detailed.unwrap_or(false) {
            ClaimedAmountView::Detailed(AggregatedTokenAmountView::default())
        } else {
            ClaimedAmountView::Total(U128(0))
        };

        self.claim_jars_internal(jar_ids, amount, accumulator)
    }
}

impl Contract {
    fn claim_jars_internal(
        &mut self,
        jar_ids: Vec<JarIdView>,
        amount: Option<U128>,
        accumulator: &mut ClaimedAmountView,
    ) -> PromiseOrValue<ClaimedAmountView> {
        let account_id = env::predecessor_account_id();
        let now = env::block_timestamp_ms();

        let unlocked_jars: Vec<Jar> = self
            .account_jars(&account_id)
            .iter()
            .filter(|jar| !jar.is_pending_withdraw && jar_ids.contains(&U32(jar.id)))
            .cloned()
            .collect();

        let mut event_data: Vec<ClaimEventItem> = vec![];

        for jar in &unlocked_jars {
            let product = self.get_product(&jar.product_id);
            let available_interest = jar.get_interest(product, now);
            let interest_to_claim = amount.map_or(available_interest, |amount| {
                cmp::min(available_interest, amount.0 - accumulator.get_total().0)
            });

            if interest_to_claim > 0 {
                self.get_jar_mut_internal(&jar.account_id, jar.id)
                    .claim(available_interest, interest_to_claim, now)
                    .lock();

                accumulator.add(jar.id, interest_to_claim);

                event_data.push(ClaimEventItem {
                    id: jar.id,
                    interest_to_claim: U128(interest_to_claim),
                });
            }
        }

        if accumulator.get_total().0 > 0 {
            self.claim_interest(
                &account_id,
                accumulator.clone(),
                unlocked_jars,
                EventKind::Claim(event_data),
            )
        } else {
            PromiseOrValue::Value(accumulator.clone())
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
        event: EventKind,
    ) -> PromiseOrValue<ClaimedAmountView> {
        PromiseOrValue::Value(self.after_claim_internal(
            claimed_amount,
            jars_before_transfer,
            event,
            crate::common::test_data::get_test_future_success(),
        ))
    }

    #[cfg(not(test))]
    fn claim_interest(
        &mut self,
        account_id: &AccountId,
        claimed_amount: ClaimedAmountView,
        jars_before_transfer: Vec<Jar>,
        event: EventKind,
    ) -> PromiseOrValue<ClaimedAmountView> {
        use crate::ft_interface::FungibleTokenInterface;
        self.ft_contract()
            .transfer(account_id, claimed_amount.get_total().0, "claim", &None)
            .then(after_claim_call(claimed_amount, jars_before_transfer, event))
            .into()
    }

    fn after_claim_internal(
        &mut self,
        claimed_amount: ClaimedAmountView,
        jars_before_transfer: Vec<Jar>,
        event: EventKind,
        is_promise_success: bool,
    ) -> ClaimedAmountView {
        if is_promise_success {
            for jar_before_transfer in jars_before_transfer {
                let jar = self.get_jar_mut_internal(&jar_before_transfer.account_id, jar_before_transfer.id);

                jar.unlock();

                if let Some(ref cache) = jar.cache {
                    if cache.interest == 0 && jar.principal == 0 {
                        self.delete_jar(&jar_before_transfer.account_id, jar_before_transfer.id);
                    }
                }
            }

            emit(event);

            claimed_amount
        } else {
            for jar_before_transfer in jars_before_transfer {
                let account_id = jar_before_transfer.account_id.clone();
                let jar_id = jar_before_transfer.id;

                *self.get_jar_mut_internal(&account_id, jar_id) = jar_before_transfer.unlocked();
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
        event: EventKind,
    ) -> ClaimedAmountView {
        self.after_claim_internal(claimed_amount, jars_before_transfer, event, is_promise_success())
    }
}

#[cfg(not(test))]
fn after_claim_call(
    claimed_amount: ClaimedAmountView,
    jars_before_transfer: Vec<Jar>,
    event: EventKind,
) -> crate::Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(crate::common::gas_data::GAS_FOR_AFTER_CLAIM)
        .after_claim(claimed_amount, jars_before_transfer, event)
}
