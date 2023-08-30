use near_sdk::{env, near_bindgen};

use crate::{
    event::{emit, EventKind::ApplyPenalty, PenaltyData},
    jar::model::JarIndex,
    product::model::Apy,
    Contract, ContractExt,
};

/// The `PenaltyApi` trait provides methods for applying or canceling penalties on premium jars within the smart contract.
pub trait PenaltyApi {
    /// Sets the penalty status for a specified jar.
    ///
    /// This method allows the contract manager to apply or cancel a penalty for a premium jar. Premium jars are those associated
    /// with products having Downgradable APY. When a user violates the terms of a premium product and a penalty is applied, the
    /// interest for the jar is calculated using a downgraded APY rate. If the terms are no longer violated, the penalty can be canceled.
    ///
    /// # Arguments
    ///
    /// * `jar_index` - The index of the jar for which the penalty status is being modified.
    /// * `value` - A boolean value indicating whether the penalty should be applied (`true`) or canceled (`false`).
    ///
    /// # Panics
    ///
    /// This method will panic if the jar's associated product has a constant APY rather than a downgradable APY.
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

        emit(ApplyPenalty(PenaltyData {
            index: jar_index,
            is_applied: value,
        }));
    }
}
