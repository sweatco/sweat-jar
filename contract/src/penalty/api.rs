use near_sdk::{env, near_bindgen};
use crate::*;
use crate::event::{emit, PenaltyData};
use crate::event::EventKind::ApplyPenalty;
use crate::jar::model::JarIndex;
use crate::product::model::Apy;

pub trait PenaltyApi {
    fn set_penalty(&mut self, jar_index: JarIndex, value: bool);
}

#[near_bindgen]
impl PenaltyApi for Contract {
    fn set_penalty(&mut self, jar_index: JarIndex, value: bool) {
        self.assert_manager();

        let jar = self.get_jar_internal(jar_index);
        let product = self.get_product(&jar.product_id);

        match product.apy {
            Apy::Downgradable(_) => {
                let updated_jar = jar.with_penalty_applied(value);
                self.jars.replace(jar.index, updated_jar);
            }
            _ => env::panic_str("Penalty is not applicable"),
        };

        emit(ApplyPenalty(PenaltyData { index: jar_index, is_applied: value }));
    }
}