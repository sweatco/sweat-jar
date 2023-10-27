#![cfg(feature = "integration-test")]

use near_sdk::{env, near_bindgen, Timestamp};

use crate::{Contract, ContractExt};

pub trait IntegrationTestMethods {
    fn block_timestamp_ms(&self) -> Timestamp;
}

#[near_bindgen]
impl IntegrationTestMethods for Contract {
    fn block_timestamp_ms(&self) -> Timestamp {
        env::block_timestamp_ms()
    }
}
