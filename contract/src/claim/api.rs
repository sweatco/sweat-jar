use std::cmp;

use near_sdk::{env, ext_contract, is_promise_success, json_types::U128, near_bindgen, PromiseOrValue};

use crate::{
    common::{TokenAmount, GAS_FOR_AFTER_CLAIM},
    event::{emit, ClaimEventItem, EventKind},
    ft_interface::FungibleTokenInterface,
    jar::model::{Jar, JarID},
    Contract, ContractExt, Promise,
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
    /// * `jar_ids` - A `Vec<JarID>` containing the indices of the deposit jars from which interest is being claimed.
    /// * `amount` - An optional `TokenAmount` specifying the desired amount of tokens to claim. If provided, the method
    ///              will attempt to claim this specific amount of tokens. If not provided or if the specified amount
    ///              is greater than the total available interest in the provided jars, the method will claim the maximum
    ///              available amount.
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<TokenAmount>` representing the amount of tokens claimed. If the total available interest
    /// across the specified jars is zero or the provided `amount` is zero, the returned value will also be zero.
    fn claim_jars(&mut self, jar_ids: Vec<JarID>, amount: Option<U128>) -> PromiseOrValue<U128>;
}

#[ext_contract(ext_self)]
pub trait ClaimCallbacks {
    fn after_claim(&mut self, claimed_amount: U128, jars_before_transfer: Vec<Jar>, event: EventKind) -> U128;
}

#[near_bindgen]
impl ClaimApi for Contract {
    fn claim_total(&mut self) -> PromiseOrValue<U128> {
        let account_id = env::predecessor_account_id();
        let jar_ids = self.account_jars(&account_id).into_iter().map(|a| a.id).collect();
        self.claim_jars(jar_ids, None)
    }

    fn claim_jars(&mut self, jar_ids: Vec<JarID>, amount: Option<U128>) -> PromiseOrValue<U128> {
        let account_id = env::predecessor_account_id();
        let now = env::block_timestamp_ms();

        let jars = self.account_jars(&account_id);

        let unlocked_jars: Vec<Jar> = jars
            .into_iter()
            .filter(|jar| !jar.is_pending_withdraw && jar_ids.contains(&jar.id))
            .collect();

        let mut total_interest_to_claim: TokenAmount = 0;

        let mut event_data: Vec<ClaimEventItem> = vec![];

        for jar in &unlocked_jars {
            let product = self.get_product(&jar.product_id);
            let available_interest = jar.get_interest(product, now);
            let interest_to_claim = amount.map_or(available_interest, |amount| {
                cmp::min(available_interest, amount.0 - total_interest_to_claim)
            });

            self.get_jar_mut_internal(&jar.account_id, jar.id)
                .claim(available_interest, interest_to_claim, now)
                .lock();

            if interest_to_claim > 0 {
                total_interest_to_claim += interest_to_claim;

                event_data.push(ClaimEventItem {
                    index: jar.id,
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
            for jar_before_transfer in jars_before_transfer {
                self.get_jar_mut_internal(&jar_before_transfer.account_id, jar_before_transfer.id)
                    .unlock();
            }

            emit(event);

            claimed_amount
        } else {
            for jar_before_transfer in jars_before_transfer {
                let account_id = jar_before_transfer.account_id.clone();
                let jar_id = jar_before_transfer.id;

                *self.get_jar_mut_internal(&account_id, jar_id) = jar_before_transfer.unlocked();
            }

            U128(0)
        }
    }
}

fn after_claim_call(claimed_amount: U128, jars_before_transfer: Vec<Jar>, event: EventKind) -> Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(GAS_FOR_AFTER_CLAIM)
        .after_claim(claimed_amount, jars_before_transfer, event)
}
