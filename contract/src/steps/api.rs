use near_sdk::{env, json_types::U64, near_bindgen, AccountId};
use sweat_jar_model::api::StepsApi;

use crate::{
    event::{emit, EventKind, StepsData},
    jar::model::JarCache,
    Contract, ContractExt,
};

#[near_bindgen]
impl StepsApi for Contract {
    fn record_steps(&mut self, timestamp: U64, batch: Vec<(AccountId, u32)>) {
        let mut event = vec![];

        for (account, steps) in batch {
            self.migrate_account_jars_if_needed(account.clone());

            let account_jars = self.account_jars.entry(account.clone()).or_default();

            for jar in &mut account_jars.jars {
                let product = self
                    .products
                    .get(&jar.product_id)
                    .unwrap_or_else(|| env::panic_str(&format!("Product '{}' doesn't exist", jar.product_id)));

                if !product.is_steps_product() {
                    continue;
                }

                let interest = jar.get_interest(steps, &product, timestamp.into()).0;

                jar.cache = Some(JarCache {
                    updated_at: timestamp.into(),
                    interest,
                });
            }

            event.push(StepsData {
                account_id: account,
                steps: steps.into(),
            })
        }

        emit(EventKind::RecordSteps(event));
    }
}
