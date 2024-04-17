use near_sdk::{env, near_bindgen, AccountId};
use sweat_jar_model::{api::PenaltyApi, jar::JarIdView};

use crate::{
    event::{
        emit, BatchPenaltyData,
        EventKind::{ApplyPenalty, BatchApplyPenalty},
        PenaltyData,
    },
    product::model::Apy,
    Contract, ContractExt, JarsStorage,
};

#[near_bindgen]
impl PenaltyApi for Contract {
    fn set_penalty(&mut self, account_id: AccountId, jar_id: JarIdView, value: bool) {
        self.assert_manager();

        self.migrate_account_jars_if_needed(account_id.clone());

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
            self.migrate_account_jars_if_needed(account_id.clone());

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
                jar.apply_penalty(&product, value, now);

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
