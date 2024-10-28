#![cfg(feature = "integration-test")]

use near_sdk::{env, near_bindgen, AccountId, Timestamp};
use sweat_jar_model::{api::IntegrationTestMethods, ProductId};

use crate::{Contract, ContractExt};

#[mutants::skip]
#[near_bindgen]
impl IntegrationTestMethods for Contract {
    fn block_timestamp_ms(&self) -> Timestamp {
        env::block_timestamp_ms()
    }

    fn bulk_create_jars(&mut self, account_id: AccountId, product_id: ProductId, principal: u128, number_of_jars: u16) {
        todo!()
        // self.assert_manager();
        // let now = env::block_timestamp_ms();
        // (0..number_of_jars)
        //     .for_each(|_| self.create_jar_for_integration_tests(&account_id, &product_id, principal, now));
    }
}
