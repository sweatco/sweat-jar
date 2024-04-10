#![cfg(feature = "integration-test")]

use near_sdk::{env, near_bindgen, Timestamp};
use sweat_jar_model::api::IntegrationTestMethods;

use crate::{Contract, ContractExt};

#[near_bindgen]
impl IntegrationTestMethods for Contract {
    #[mutants::skip]
    fn block_timestamp_ms(&self) -> Timestamp {
        env::block_timestamp_ms()
    }
}
