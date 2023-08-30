use std::cmp;

use near_sdk::{env, ext_contract, is_promise_success, json_types::U128, near_bindgen, PromiseOrValue};

use crate::{
    common::TokenAmount,
    event::{emit, ClaimEventItem, EventKind},
    ft_interface::{FungibleTokenInterface, GAS_FOR_AFTER_TRANSFER},
    jar::model::{Jar, JarIndex},
    Contract, ContractExt, Gas, Promise,
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

    /// Claims interest from specific deposit jars with provided indices.
    ///
    /// # Arguments
    ///
    /// * `jar_indices` - A `Vec<JarIndex>` containing the indices of the deposit jars from which interest is being claimed.
    /// * `amount` - An optional `TokenAmount` specifying the desired amount of tokens to claim. If provided, the method
    ///              will attempt to claim this specific amount of tokens. If not provided or if the specified amount
    ///              is greater than the total available interest in the provided jars, the method will claim the maximum
    ///              available amount.
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<TokenAmount>` representing the amount of tokens claimed. If the total available interest
    /// across the specified jars is zero or the provided `amount` is zero, the returned value will also be zero.
    fn claim_jars(&mut self, jar_indices: Vec<JarIndex>, amount: Option<U128>) -> PromiseOrValue<U128>;
}

#[ext_contract(ext_self)]
pub trait ClaimCallbacks {
    fn after_claim(&mut self, claimed_amount: U128, jars_before_transfer: Vec<Jar>, event: EventKind) -> U128;
}

#[near_bindgen]
impl ClaimApi for Contract {
    fn claim_total(&mut self) -> PromiseOrValue<U128> {
        let account_id = env::predecessor_account_id();
        let jar_indices = self.account_jar_ids(&account_id);

        self.claim_jars(jar_indices, None)
    }

    fn claim_jars(&mut self, jar_indices: Vec<JarIndex>, amount: Option<U128>) -> PromiseOrValue<U128> {
        let account_id = env::predecessor_account_id();
        let now = env::block_timestamp_ms();

        let unlocked_jars: Vec<Jar> = jar_indices
            .into_iter()
            .map(|index| self.get_jar_internal(index))
            .filter(|jar| !jar.is_pending_withdraw && jar.account_id == account_id)
            .collect();

        let mut total_interest_to_claim: TokenAmount = 0;

        let mut event_data: Vec<ClaimEventItem> = vec![];

        for jar in &unlocked_jars {
            let product = self.get_product(&jar.product_id);
            let available_interest = jar.get_interest(&product, now);
            let interest_to_claim = amount.map_or(available_interest, |amount| {
                cmp::min(available_interest, amount.0 - total_interest_to_claim)
            });

            let updated_jar = jar.claimed(available_interest, interest_to_claim, now).locked();
            self.jars.replace(jar.index, updated_jar);

            if interest_to_claim > 0 {
                total_interest_to_claim += interest_to_claim;

                event_data.push(ClaimEventItem {
                    index: jar.index,
                    interest_to_claim: U128(interest_to_claim),
                });
            }
        }

        if total_interest_to_claim > 0 {
            self.ft_contract()
                .transfer(&account_id, total_interest_to_claim, "claim", &None)
                .then(after_claim_call(
                    U128(total_interest_to_claim),
                    unlocked_jars,
                    EventKind::Claim(event_data),
                ))
                .into()
        } else {
            PromiseOrValue::Value(U128(0))
        }
    }
}

#[near_bindgen]
impl ClaimCallbacks for Contract {
    #[private]
    fn after_claim(&mut self, claimed_amount: U128, jars_before_transfer: Vec<Jar>, event: EventKind) -> U128 {
        if is_promise_success() {
            for jar_before_transfer in jars_before_transfer.iter() {
                let jar = self.get_jar_internal(jar_before_transfer.index);

                self.jars.replace(jar_before_transfer.index, jar.unlocked());
            }

            emit(event);

            claimed_amount
        } else {
            for jar_before_transfer in jars_before_transfer.iter() {
                self.jars
                    .replace(jar_before_transfer.index, jar_before_transfer.unlocked());
            }

            U128(0)
        }
    }
}

fn after_claim_call(claimed_amount: U128, jars_before_transfer: Vec<Jar>, event: EventKind) -> Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(Gas::from(GAS_FOR_AFTER_TRANSFER))
        .after_claim(claimed_amount, jars_before_transfer, event)
}
