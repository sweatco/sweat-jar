use model::jar::JarIdView;
use near_sdk::{env, near_bindgen, AccountId};

use crate::{
    event::{
        emit, BatchPenaltyData,
        EventKind::{ApplyPenalty, BatchApplyPenalty},
        PenaltyData,
    },
    product::model::Apy,
    Contract, ContractExt, JarsStorage,
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
    /// * `account_id` - The account of user which owns this jar.
    /// * `jar_id` - The ID of the jar for which the penalty status is being modified.
    /// * `value` - A boolean value indicating whether the penalty should be applied (`true`) or canceled (`false`).
    ///
    /// # Panics
    ///
    /// This method will panic if the jar's associated product has a constant APY rather than a downgradable APY.
    fn set_penalty(&mut self, account_id: AccountId, jar_id: JarIdView, value: bool);

    /// Batched version of `set_penalty`
    ///
    /// # Arguments
    ///
    /// * `jars` - List of Account IDs and their Jar IDs to which penalty must be applied.
    /// * `value` - A boolean value indicating whether the penalty should be applied (`true`) or canceled (`false`).
    ///
    /// # Panics
    ///
    /// This method will panic if the jar's associated product has a constant APY rather than a downgradable APY.
    fn batch_set_penalty(&mut self, jars: Vec<(AccountId, Vec<JarIdView>)>, value: bool);
}

#[near_bindgen]
impl PenaltyApi for Contract {
    fn set_penalty(&mut self, account_id: AccountId, jar_id: JarIdView, value: bool) {
        self.assert_manager();

        let jar_id = jar_id.0;
        let jar = self.get_jar_internal(&account_id, jar_id);
        let product = self.get_product(&jar.product_id).clone();
        let now = env::block_timestamp_ms();

        assert_penalty_apy(&product.apy);
        self.get_jar_mut_internal(&account_id, jar_id)
            .apply_penalty(&product, value, now);

        emit(ApplyPenalty(PenaltyData {
            id: jar_id,
            is_applied: value,
            timestamp: now,
        }));
    }

    fn batch_set_penalty(&mut self, jars: Vec<(AccountId, Vec<JarIdView>)>, value: bool) {
        self.assert_manager();

        let mut applied_jars = vec![];

        let now = env::block_timestamp_ms();

        for (account_id, jars) in jars {
            let account_jars = self
                .account_jars
                .get_mut(&account_id)
                .unwrap_or_else(|| env::panic_str(&format!("Account '{account_id}' doesn't exist")));

            for jar_id in jars {
                let jar_id = jar_id.0;

                let jar = account_jars.get_jar_mut(jar_id);

                let product = self
                    .products
                    .get(&jar.product_id)
                    .unwrap_or_else(|| env::panic_str(&format!("Product '{}' doesn't exist", jar.product_id)));

                assert_penalty_apy(&product.apy);
                jar.apply_penalty(product, value, now);

                applied_jars.push(jar_id);
            }
        }

        emit(BatchApplyPenalty(BatchPenaltyData {
            jars: applied_jars,
            is_applied: value,
            timestamp: now,
        }));
    }
}

fn assert_penalty_apy(apy: &Apy) {
    match apy {
        Apy::Constant(_) => env::panic_str("Penalty is not applicable for constant APY"),
        Apy::Downgradable(_) => (),
    }
}
