use near_sdk::{env, json_types::U64, near_bindgen, AccountId};
use sweat_jar_model::{api::ScoreApi, U32};

use crate::{
    event::{emit, EventKind, ScoreData},
    jar::model::JarCache,
    Contract, ContractExt,
};

#[near_bindgen]
impl ScoreApi for Contract {
    fn record_score(&mut self, timestamp: U64, batch: Vec<(AccountId, u16)>) {
        let mut event = vec![];

        for (account, new_score) in batch {
            self.migrate_account_jars_if_needed(&account);

            let score = self.account_score.entry(account.clone()).or_default();
            let account_jars = self.account_jars.entry(account.clone()).or_default();

            for jar in &mut account_jars.jars {
                let product = self
                    .products
                    .get(&jar.product_id)
                    .unwrap_or_else(|| env::panic_str(&format!("Product '{}' doesn't exist", jar.product_id)));

                if !product.is_score_product() {
                    continue;
                }

                let (interest, remainder) = jar.get_interest(score, &product, timestamp.into());

                jar.claim_remainder = remainder;

                jar.cache = Some(JarCache {
                    updated_at: timestamp.into(),
                    interest,
                });
            }

            score.score = new_score;
            score.last_update = timestamp.into();

            event.push(ScoreData {
                account_id: account,
                score: U32(new_score.into()),
            });
        }

        emit(EventKind::RecordScore(event));
    }
}
