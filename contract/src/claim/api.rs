use std::cmp;

use model::{jar::JarIdView, TokenAmount, U32};
use near_sdk::{env, ext_contract, is_promise_success, json_types::U128, near_bindgen, AccountId, PromiseOrValue};

use crate::{
    event::{emit, ClaimEventItem, EventKind},
    jar::{model::Jar, view::AggregatedTokenAmountView},
    Contract, ContractExt,
};

/// The `ClaimApi` trait defines methods for claiming interest from jars within the smart contract.
pub trait ClaimApi {
    /// Claims all available interest from all deposit jars belonging to the calling account.
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<TokenAmount>` representing the amount of tokens claimed. If the total available
    /// interest across all jars is zero, the returned value will also be zero.
    fn claim_total(&mut self) -> PromiseOrValue<U128>;

    fn claim_total_detailed(&mut self) -> PromiseOrValue<AggregatedTokenAmountView>;

    /// Claims interest from specific deposit jars with provided IDs.
    ///
    /// # Arguments
    ///
    /// * `jar_ids` - A `Vec<JarId>` containing the IDs of the deposit jars from which interest is being claimed.
    /// * `amount` - An optional `TokenAmount` specifying the desired amount of tokens to claim. If provided, the method
    ///              will attempt to claim this specific amount of tokens. If not provided or if the specified amount
    ///              is greater than the total available interest in the provided jars, the method will claim the maximum
    ///              available amount.
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<TokenAmount>` representing the amount of tokens claimed. If the total available interest
    /// across the specified jars is zero or the provided `amount` is zero, the returned value will also be zero.
    fn claim_jars(
        &mut self,
        jar_ids: Vec<JarIdView>,
        amount: Option<U128>,
    ) -> PromiseOrValue<AggregatedTokenAmountView>;
}

#[ext_contract(ext_self)]
pub trait ClaimCallbacks {
    fn after_claim(&mut self, claimed_amount: U128, jars_before_transfer: Vec<Jar>, event: EventKind) -> U128;
}

#[near_bindgen]
impl ClaimApi for Contract {
    fn claim_total(&mut self) -> PromiseOrValue<U128> {
        let account_id = env::predecessor_account_id();
        let jar_ids = self.account_jars(&account_id).iter().map(|a| U32(a.id)).collect();

        self.claim_jars_internal(jar_ids, None, |acc: U128, value: U128| U128(acc.0 + value.0))
    }

    // fn claim_jars(
    //     &mut self,
    //     jar_ids: Vec<JarIdView>,
    //     amount: Option<U128>,
    // ) -> PromiseOrValue<AggregatedTokenAmountView> {
    //     let account_id = env::predecessor_account_id();
    //     let now = env::block_timestamp_ms();
    //
    //     let unlocked_jars: Vec<Jar> = self
    //         .account_jars(&account_id)
    //         .iter()
    //         .filter(|jar| !jar.is_pending_withdraw && jar_ids.contains(&U32(jar.id)))
    //         .cloned()
    //         .collect();
    //
    //     let mut total_interest_to_claim: TokenAmount = 0;
    //
    //     let mut event_data: Vec<ClaimEventItem> = vec![];
    //
    //     for jar in &unlocked_jars {
    //         let product = self.get_product(&jar.product_id);
    //         let available_interest = jar.get_interest(product, now);
    //         let interest_to_claim = amount.map_or(available_interest, |amount| {
    //             cmp::min(available_interest, amount.0 - total_interest_to_claim)
    //         });
    //
    //         if interest_to_claim > 0 {
    //             self.get_jar_mut_internal(&jar.account_id, jar.id)
    //                 .claim(available_interest, interest_to_claim, now)
    //                 .lock();
    //
    //             total_interest_to_claim += interest_to_claim;
    //
    //             event_data.push(ClaimEventItem {
    //                 id: jar.id,
    //                 interest_to_claim: U128(interest_to_claim),
    //             });
    //         }
    //     }
    //
    //     if total_interest_to_claim > 0 {
    //         self.claim_interest(
    //             &account_id,
    //             U128(total_interest_to_claim),
    //             unlocked_jars,
    //             EventKind::Claim(event_data),
    //         )
    //     } else {
    //         PromiseOrValue::Value(U128(0))
    //     }
    // }
}

impl Contract {
    fn claim_jars_internal<F, T>(
        &mut self,
        jar_ids: Vec<JarIdView>,
        amount: Option<U128>,
        aggregation_function: F,
    ) -> PromiseOrValue<T>
    where
        T: TokenAmountRepresentation + Default,
        F: FnMut(T, T) -> T,
    {
        let account_id = env::predecessor_account_id();
        let now = env::block_timestamp_ms();

        let unlocked_jars: Vec<Jar> = self
            .account_jars(&account_id)
            .iter()
            .filter(|jar| !jar.is_pending_withdraw && jar_ids.contains(&U32(jar.id)))
            .cloned()
            .collect();

        let mut total_interest_to_claim: T = Default::default();

        let mut event_data: Vec<ClaimEventItem> = vec![];

        for jar in &unlocked_jars {
            let product = self.get_product(&jar.product_id);
            let available_interest = jar.get_interest(product, now);
            let interest_to_claim = amount.map_or(available_interest, |amount| {
                cmp::min(available_interest, amount.0 - total_interest_to_claim.get_total().0)
            });

            if interest_to_claim > 0 {
                self.get_jar_mut_internal(&jar.account_id, jar.id)
                    .claim(available_interest, interest_to_claim, now)
                    .lock();

                total_interest_to_claim = aggregation_function(total_interest_to_claim, interest_to_claim);

                event_data.push(ClaimEventItem {
                    id: jar.id,
                    interest_to_claim: U128(interest_to_claim),
                });
            }
        }

        if total_interest_to_claim.get_total().0 > 0 {
            self.claim_interest(
                &account_id,
                total_interest_to_claim,
                unlocked_jars,
                EventKind::Claim(event_data),
            )
        } else {
            PromiseOrValue::Value(T::default())
        }
    }
}

impl Contract {
    #[cfg(test)]
    fn claim_interest<T>(
        &mut self,
        _account_id: &AccountId,
        value_to_return: T,
        jars_before_transfer: Vec<Jar>,
        event: EventKind,
    ) -> PromiseOrValue<T>
    where
        T: TokenAmountRepresentation,
    {
        PromiseOrValue::Value(self.after_claim_internal(
            value_to_return,
            jars_before_transfer,
            event,
            crate::common::test_data::get_test_future_success(),
        ))
    }

    #[cfg(not(test))]
    fn claim_interest<T: TokenAmountRepresentation>(
        &mut self,
        account_id: &AccountId,
        claimed_amount: T,
        jars_before_transfer: Vec<Jar>,
        event: EventKind,
    ) -> PromiseOrValue<T>
    where
        T: TokenAmountRepresentation,
    {
        use crate::ft_interface::FungibleTokenInterface;
        self.ft_contract()
            .transfer(account_id, claimed_amount.get_total().0, "claim", &None)
            .then(after_claim_call(
                claimed_amount.get_total(),
                jars_before_transfer,
                event,
            ))
            .into()
    }

    fn after_claim_internal<T: TokenAmountRepresentation>(
        &mut self,
        value_to_return: T,
        jars_before_transfer: Vec<Jar>,
        event: EventKind,
        is_promise_success: bool,
    ) -> T {
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

            value_to_return
        } else {
            for jar_before_transfer in jars_before_transfer {
                let account_id = jar_before_transfer.account_id.clone();
                let jar_id = jar_before_transfer.id;

                *self.get_jar_mut_internal(&account_id, jar_id) = jar_before_transfer.unlocked();
            }

            value_to_return
        }
    }
}

#[near_bindgen]
impl ClaimCallbacks for Contract {
    #[private]
    fn after_claim(&mut self, claimed_amount: U128, jars_before_transfer: Vec<Jar>, event: EventKind) -> U128 {
        self.after_claim_internal(claimed_amount, jars_before_transfer, event, is_promise_success())
    }
}

#[cfg(not(test))]
fn after_claim_call(claimed_amount: U128, jars_before_transfer: Vec<Jar>, event: EventKind) -> crate::Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(crate::common::gas_data::GAS_FOR_AFTER_CLAIM)
        .after_claim(claimed_amount, jars_before_transfer, event)
}

trait TokenAmountRepresentation {
    fn get_total(&self) -> U128;
}

impl TokenAmountRepresentation for U128 {
    fn get_total(&self) -> U128 {
        self.clone()
    }
}

impl TokenAmountRepresentation for AggregatedTokenAmountView {
    fn get_total(&self) -> U128 {
        self.total
    }
}

impl Default for U128 {
    fn default() -> Self {
        Self(0)
    }
}
