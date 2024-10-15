use near_sdk::{near_bindgen, AccountId};
use sweat_jar_model::{api::ScoreApi, Score, U32, UTC};

use crate::{
    event::{emit, EventKind, ScoreData},
    Contract, ContractExt,
};

#[near_bindgen]
impl ScoreApi for Contract {
    fn record_score(&mut self, batch: Vec<(AccountId, Vec<(Score, UTC)>)>) {
        self.assert_manager();

        let mut event = vec![];

        for (account_id, _) in batch.iter() {
            self.migrate_account_if_needed(account_id);
            self.update_account_cache(account_id);
        }

        for (account_id, new_score) in batch {
            let account = self.get_account_mut(&account_id);

            // Convert a record to user timezone
            let converted_score = new_score
                .iter()
                .map(|score| (score.0, account.score.timezone.adjust(score.1)))
                .collect();

            account.score.update(converted_score);

            event.push(ScoreData {
                account_id,
                score: new_score
                    .into_iter()
                    .map(|(score, timestamp)| (U32(score.into()), timestamp))
                    .collect(),
            });
        }

        emit(EventKind::RecordScore(event));
    }
}
