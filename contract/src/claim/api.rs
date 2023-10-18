use std::cmp;

use model::{jar::JarIdView, TokenAmount, U32};
use near_sdk::{env, ext_contract, is_promise_success, json_types::U128, near_bindgen, PromiseOrValue};

use crate::jar::model::{ClaimData, JarClaimData};
use crate::{
    event::{emit, ClaimEventItem, EventKind},
    jar::model::Jar,
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
    fn claim_jars(&mut self, jar_ids: Vec<JarIdView>, amount: Option<U128>) -> PromiseOrValue<U128>;
}

#[ext_contract(ext_self)]
pub trait ClaimCallbacks {
    fn after_claim(&mut self, claim_data: ClaimData, event: EventKind) -> U128;
}

#[near_bindgen]
impl ClaimApi for Contract {
    fn claim_total(&mut self) -> PromiseOrValue<U128> {
        let account_id = env::predecessor_account_id();
        let jar_ids = self.account_jars(&account_id).iter().map(|a| U32(a.id)).collect();
        self.claim_jars(jar_ids, None)
    }

    fn claim_jars(&mut self, jar_ids: Vec<JarIdView>, amount: Option<U128>) -> PromiseOrValue<U128> {
        let account_id = env::predecessor_account_id();
        let now = env::block_timestamp_ms();

        let unlocked_jars: Vec<Jar> = self
            .account_jars(&account_id)
            .iter()
            .filter(|jar| !jar.is_pending_withdraw && jar_ids.contains(&U32(jar.id)))
            .cloned()
            .collect();

        let mut total_interest_to_claim: TokenAmount = 0;

        let mut event_data: Vec<ClaimEventItem> = vec![];
        let mut claim_jars: Vec<JarClaimData> = vec![];

        for jar in &unlocked_jars {
            let product = self.get_product(&jar.product_id);
            let available_interest = jar.get_interest(product, now);
            let interest_to_claim = amount.map_or(available_interest, |amount| {
                cmp::min(available_interest, amount.0 - total_interest_to_claim)
            });

            if interest_to_claim == 0 {
                continue;
            }

            let claimed_balance_before = jar.claimed_balance;

            let jar =
                self.get_jar_mut_internal(&jar.account_id, jar.id)
                    .claim(available_interest, interest_to_claim, now);

            jar.lock();

            total_interest_to_claim += interest_to_claim;

            event_data.push(ClaimEventItem {
                id: jar.id,
                interest_to_claim: U128(interest_to_claim),
            });

            claim_jars.push(JarClaimData {
                id: jar.id,
                available_interest,
                interest_to_claim,
                claimed_balance_before,
            });
        }

        if total_interest_to_claim > 0 {
            self.claim_interest(
                ClaimData {
                    timestamp: now,
                    total_claimed_amount: total_interest_to_claim,
                    account_id,
                    jars: claim_jars,
                },
                EventKind::Claim(event_data),
            )
        } else {
            PromiseOrValue::Value(U128(0))
        }
    }
}

impl Contract {
    #[cfg(test)]
    fn claim_interest(&mut self, claim_data: ClaimData, event: EventKind) -> PromiseOrValue<U128> {
        PromiseOrValue::Value(self.after_claim_internal(
            claim_data,
            event,
            crate::common::test_data::get_test_future_success(),
        ))
    }

    #[cfg(not(test))]
    fn claim_interest(&mut self, claim_data: ClaimData, event: EventKind) -> PromiseOrValue<U128> {
        use crate::ft_interface::FungibleTokenInterface;
        self.ft_contract()
            .transfer(&claim_data.account_id, claim_data.total_claimed_amount, "claim", &None)
            .then(after_claim_call(claim_data, event))
            .into()
    }

    fn after_claim_internal(&mut self, claim_data: ClaimData, event: EventKind, is_promise_success: bool) -> U128 {
        if !is_promise_success {
            for claimed_jar in claim_data.jars {
                let jar = self.get_jar_mut_internal(&claim_data.account_id, claimed_jar.id);
                jar.unlock();
                jar.claimed_balance = claimed_jar.claimed_balance_before;
            }

            return U128(0);
        }

        for claimed_jar in claim_data.jars {
            let jar = self.get_jar_mut_internal(&claim_data.account_id, claimed_jar.id);
            jar.unlock();

            let jar = jar.clone();

            if let Some(ref cache) = jar.cache {
                if cache.interest == 0 && jar.principal == 0 {
                    self.delete_jar(jar);
                }
            }
        }

        emit(event);

        claim_data.total_claimed_amount.into()
    }
}

#[near_bindgen]
impl ClaimCallbacks for Contract {
    #[private]
    fn after_claim(&mut self, claim_data: ClaimData, event: EventKind) -> U128 {
        self.after_claim_internal(claim_data, event, is_promise_success())
    }
}

#[cfg(not(test))]
fn after_claim_call(claim_data: ClaimData, event: EventKind) -> crate::Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(crate::common::gas_data::GAS_FOR_AFTER_CLAIM)
        .after_claim(claim_data, event)
}
