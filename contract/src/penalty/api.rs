use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{env, near, AccountId};
use sweat_jar_model::{api::PenaltyApi, jar::JarIdView};

use crate::{
    event::{
        emit, BatchPenaltyData,
        EventKind::{ApplyPenalty, BatchApplyPenalty},
        PenaltyData,
    },
    product::model::Apy,
    Contract, ContractExt, Roles,
};

#[near]
impl PenaltyApi for Contract {
    #[access_control_any(roles(Roles::Maintainer))]
    fn set_penalty(&mut self, account_id: AccountId, jar_id: JarIdView, value: bool) {
        self.migrate_account_if_needed(&account_id);

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

    #[access_control_any(roles(Roles::Maintainer))]
    fn batch_set_penalty(&mut self, jars: Vec<(AccountId, Vec<JarIdView>)>, value: bool) {
        let mut applied_jars = vec![];

        let now = env::block_timestamp_ms();

        for (account_id, jars) in jars {
            self.migrate_account_if_needed(&account_id);

            for jar_id in jars {
                let jar_id = jar_id.0;

                let jar = self.get_jar_internal(&account_id, jar_id);
                let product = self.get_product(&jar.product_id);

                assert_penalty_apy(&product.apy);

                self.get_jar_mut_internal(&account_id, jar_id)
                    .apply_penalty(&product, value, now);

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
