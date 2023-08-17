use std::cmp;

use near_sdk::{env, ext_contract, is_promise_success, near_bindgen, PromiseOrValue};

use crate::*;
use crate::common::TokenAmount;
use crate::event::{ClaimEventItem, emit, EventKind};
use crate::external::GAS_FOR_AFTER_TRANSFER;
use crate::ft_interface::FungibleTokenInterface;
use crate::jar::model::{Jar, JarIndex};

/// The `ClaimApi` trait defines methods for claiming interest from jars within the smart contract.
pub trait ClaimApi {
    /// Claims all available interest from all deposit jars belonging to the calling account.
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<TokenAmount>` representing the amount of tokens claimed. If the total available
    /// interest across all jars is zero, the returned value will also be zero.
    fn claim_total(&mut self) -> PromiseOrValue<TokenAmount>;

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
    fn claim_jars(
        &mut self,
        jar_indices: Vec<JarIndex>,
        amount: Option<TokenAmount>,
    ) -> PromiseOrValue<TokenAmount>;
}

#[ext_contract(ext_self)]
pub trait ClaimCallbacks {
    fn after_claim(&mut self, jars_before_transfer: Vec<Jar>);
}

#[near_bindgen]
impl ClaimApi for Contract {
    fn claim_total(&mut self) -> PromiseOrValue<TokenAmount> {
        let account_id = env::predecessor_account_id();
        let jar_indices = self.account_jar_ids(&account_id);

        self.claim_jars(jar_indices, None)
    }

    fn claim_jars(
        &mut self,
        jar_indices: Vec<JarIndex>,
        amount: Option<TokenAmount>,
    ) -> PromiseOrValue<TokenAmount> {
        let account_id = env::predecessor_account_id();
        let now = env::block_timestamp_ms();

        let get_interest_to_claim: Box<dyn Fn(TokenAmount, TokenAmount) -> TokenAmount> = match amount {
            Some(ref a) => Box::new(|available, total| cmp::min(available, *a - total)),
            None => Box::new(|available, _| available),
        };

        let jar_ids_iter = jar_indices.iter();
        let unlocked_jars: Vec<Jar> = jar_ids_iter
            .map(|index| self.get_jar_internal(*index))
            .filter(|jar| !jar.is_pending_withdraw)
            .filter(|jar| jar.account_id == account_id)
            .collect();

        let mut total_interest_to_claim: TokenAmount = 0;

        let mut event_data: Vec<ClaimEventItem> = vec![];

        for jar in unlocked_jars.clone() {
            let product = self.get_product(&jar.product_id);
            let available_interest = jar.get_interest(&product, now);
            let interest_to_claim =
                get_interest_to_claim(available_interest, total_interest_to_claim);

            let updated_jar = jar
                .claimed(available_interest, interest_to_claim, now)
                .locked();
            self.jars.replace(jar.index, updated_jar);

            total_interest_to_claim += interest_to_claim;

            event_data.push(ClaimEventItem { index: jar.index, interest_to_claim });
        }

        emit(EventKind::Claim(event_data));

        if total_interest_to_claim > 0 {
            self.ft_contract()
                .transfer(
                    &account_id,
                    total_interest_to_claim,
                    "claim",
                    None,
                )
                .then(after_claim_call(unlocked_jars))
                .into()
        } else {
            PromiseOrValue::Value(0)
        }
    }
}

#[near_bindgen]
impl ClaimCallbacks for Contract {
    #[private]
    fn after_claim(&mut self, jars_before_transfer: Vec<Jar>) {
        if is_promise_success() {
            for jar_before_transfer in jars_before_transfer.iter() {
                let jar = self.get_jar_internal(jar_before_transfer.index);

                self.jars.replace(jar_before_transfer.index, jar.unlocked());
            }
        } else {
            for jar_before_transfer in jars_before_transfer.iter() {
                self.jars.replace(jar_before_transfer.index, jar_before_transfer.unlocked());
            }
        }
    }
}

fn after_claim_call(jars_before_transfer: Vec<Jar>) -> Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(Gas::from(GAS_FOR_AFTER_TRANSFER))
        .after_claim(jars_before_transfer)
}