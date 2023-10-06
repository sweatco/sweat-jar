use model::jar::JarIdView;
use near_sdk::{env, near_bindgen, AccountId};

use crate::{
    event::{emit, EventKind::ApplyPenalty, PenaltyData},
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
    /// * `jar_id` - The ID of the jar for which the penalty status is being modified.
    /// * `value` - A boolean value indicating whether the penalty should be applied (`true`) or canceled (`false`).
    ///
    /// # Panics
    ///
    /// This method will panic if the jar's associated product has a constant APY rather than a downgradable APY.
    fn set_penalty(&mut self, account_id: AccountId, jar_id: JarIdView, value: bool);
}

#[near_bindgen]
impl PenaltyApi for Contract {
    fn set_penalty(&mut self, account_id: AccountId, jar_id: JarIdView, value: bool) {
        self.assert_manager();

        let jar_id = jar_id.0;
        let jar = self.get_jar_internal(&account_id, jar_id);
        let product = self.get_product(&jar.product_id);

        match product.apy {
            Apy::Downgradable(_) => self.get_jar_mut_internal(&account_id, jar_id).apply_penalty(value),
            Apy::Constant(_) => env::panic_str("Penalty is not applicable for constant APY"),
        };

        emit(ApplyPenalty(PenaltyData {
            id: jar_id,
            is_applied: value,
        }));
    }
}
