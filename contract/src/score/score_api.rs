use near_sdk::{env, env::block_timestamp_ms, json_types::I64, near_bindgen, AccountId};
use sweat_jar_model::{api::ScoreApi, Score, U32, UTC};

use crate::{
    event::{emit, EventKind, ScoreData},
    jar::model::JarCache,
    Contract, ContractExt,
};

#[near_bindgen]
impl ScoreApi for Contract {
    fn record_score(&mut self, batch: Vec<(AccountId, Vec<(Score, UTC)>)>) {
        self.assert_manager();

        let mut event = vec![];

        let now = block_timestamp_ms();

        for (account, new_score) in batch {
            self.migrate_account_if_needed(&account);

            let account_jars = self.accounts.entry(account.clone()).or_default();

            assert!(
                account_jars.has_score_jars(),
                "Account '{account}' doesn't have score jars"
            );

            let score = account_jars.score.claim_score();

            for jar in &mut account_jars.jars {
                let product = self
                    .products
                    .get(&jar.product_id)
                    .unwrap_or_else(|| env::panic_str(&format!("Product '{}' doesn't exist", jar.product_id)));

                if !product.is_score_product() {
                    continue;
                }

                let (interest, remainder) = jar.get_interest(&score, &product, now);

                jar.claim_remainder = remainder;

                jar.cache = Some(JarCache {
                    updated_at: now,
                    interest,
                });
            }

            // Convert walkchain to user timezone
            let converted_score = new_score
                .iter()
                .map(|score| (score.0, account_jars.score.timezone.adjust(score.1)))
                .collect();

            account_jars.score.update(converted_score);

            event.push(ScoreData {
                account_id: account,
                score: new_score
                    .into_iter()
                    .map(|(score, timestamp)| (U32(score.into()), timestamp))
                    .collect(),
            });
        }

        emit(EventKind::RecordScore(event));
    }

    fn get_timezone(&self, account_id: AccountId) -> Option<I64> {
        self.accounts
            .get(&account_id)
            .map(|account| I64(*account.score.timezone))
    }
}
