use near_sdk::env::panic_str;
use sweat_jar_model::{api::BoosterApi, data::booster::Booster};

use crate::Contract;

impl BoosterApi for Contract {
    fn register_booster(&mut self, booster: Booster) {
        self.assert_manager();

        if self.boosters.contains(&booster.id) {
            panic_str(format!("Booster {} is already registered", &booster.id).as_str());
        }

        self.boosters.add(&booster);
    }

    fn get_boosters(&self) -> Vec<Booster> {
        self.boosters.list()
    }
}
