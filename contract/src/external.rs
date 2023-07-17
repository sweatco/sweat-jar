use near_sdk::{ext_contract, is_promise_success};

use crate::*;

// TODO: maybe calculate in runtime
pub(crate) const GAS_FOR_AFTER_TRANSFER: u64 = 20_000_000_000_000;

#[ext_contract(ext_self)]
pub trait SelfCallbacks {
    fn after_transfer(&mut self, jars_before_transfer: Vec<Jar>);
}

#[near_bindgen]
impl SelfCallbacks for Contract {
    #[private]
    fn after_transfer(&mut self, jars_before_transfer: Vec<Jar>) {
        self.after_transfer_internal(jars_before_transfer, is_promise_success());
    }
}
