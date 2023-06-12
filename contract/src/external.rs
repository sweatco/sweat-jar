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
        if is_promise_success() {
            for jar_before_transfer in jars_before_transfer.iter() {
                let jar = self.get_jar(jar_before_transfer.index);
                self.jars
                    .replace(jar_before_transfer.index, &jar.unlocked());
            }
        } else {
            for jar_before_transfer in jars_before_transfer.iter() {
                self.jars
                    .replace(jar_before_transfer.index, &jar_before_transfer.unlocked());
            }
        }
    }
}
