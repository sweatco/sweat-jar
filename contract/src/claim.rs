use std::cmp;
use near_sdk::{Balance, env, ext_contract, near_bindgen, PromiseOrValue, serde_json};
use near_sdk::serde_json::json;
use crate::*;
use crate::ft_interface::{FungibleTokenContract, FungibleTokenInterface};
use crate::jar::{Jar, JarApi, JarIndex};

#[ext_contract(ext_self)]
pub trait ClaimCallbacks {
    fn after_claim(&mut self, jars_before_transfer: Vec<Jar>);
}

pub trait ClaimApi {
    fn claim_total(&mut self) -> PromiseOrValue<Balance>;
    fn claim_jars(
        &mut self,
        jar_indices: Vec<JarIndex>,
        amount: Option<Balance>,
    ) -> PromiseOrValue<Balance>;
}

#[near_bindgen]
impl ClaimApi for Contract {
    fn claim_total(&mut self) -> PromiseOrValue<Balance> {
        let account_id = env::predecessor_account_id();
        let jar_indices = self.account_jar_ids(&account_id);

        self.claim_jars(jar_indices, None)
    }

    fn claim_jars(
        &mut self,
        jar_indices: Vec<JarIndex>,
        amount: Option<Balance>,
    ) -> PromiseOrValue<Balance> {
        let account_id = env::predecessor_account_id();
        let now = env::block_timestamp_ms();

        let get_interest_to_claim: Box<dyn Fn(Balance, Balance) -> Balance> = match amount {
            Some(ref a) => Box::new(|available, total| cmp::min(available, *a - total)),
            None => Box::new(|available, _| available),
        };

        let jar_ids_iter = jar_indices.iter();
        let unlocked_jars: Vec<Jar> = jar_ids_iter
            .map(|index| self.get_jar(*index))
            .filter(|jar| !jar.is_pending_withdraw)
            .filter(|jar| jar.account_id == account_id)
            .collect();

        let mut total_interest_to_claim: Balance = 0;

        let mut event_data: Vec<serde_json::Value> = vec![];

        for jar in unlocked_jars.clone() {
            let product = self.get_product(&jar.product_id);
            let available_interest = jar.get_interest(&product, now);
            let interest_to_claim =
                get_interest_to_claim(available_interest, total_interest_to_claim);

            let updated_jar = jar
                .claimed(available_interest, interest_to_claim, now)
                .locked();
            self.jars.replace(jar.index, &updated_jar);

            total_interest_to_claim += interest_to_claim;

            event_data.push(json!({ "index": jar.index, "interest_to_claim": interest_to_claim }));
        }

        let event = json!({
            "standard": "sweat_jar",
            "version": "0.0.1",
            "event": "claim_jars",
            "data": event_data,
        });
        env::log_str(format!("EVENT_JSON: {}", event.to_string().as_str()).as_str());

        if total_interest_to_claim > 0 {
            FungibleTokenContract::new(self.token_account_id.clone())
                .transfer(
                    account_id,
                    total_interest_to_claim,
                    after_claim_call(unlocked_jars),
                )
                .into()
        } else {
            PromiseOrValue::Value(0)
        }
    }
}

fn after_claim_call(jars_before_transfer: Vec<Jar>) -> Promise {
    ext_self::ext(env::current_account_id())
        .with_static_gas(Gas::from(GAS_FOR_AFTER_TRANSFER))
        .after_claim(jars_before_transfer)
}