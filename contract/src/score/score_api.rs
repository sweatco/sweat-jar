use near_sdk::{near_bindgen, AccountId};
use sweat_jar_model::{api::ScoreApi, Score, Timezone, U32, UTC};

use crate::{
    event::{emit, EventKind, ScoreData},
    product::model::{v2::Terms, ProductV2},
    score::Chain,
    Contract, ContractExt,
};

#[near_bindgen]
impl ScoreApi for Contract {
    fn record_score(&mut self, batch: Vec<(AccountId, Vec<(Score, UTC)>)>) {
        self.assert_manager();

        let mut event = vec![];

        for (account_id, _) in batch.iter() {
            self.assert_migrated(account_id);
        }

        for (account_id, new_score) in batch {
            self.update_account_cache(
                &account_id,
                Some(Box::new(|product: &ProductV2| {
                    matches!(product.terms, Terms::ScoreBased(_))
                })),
            );

            let account = self.get_account_mut(&account_id);
            account.score.try_claim_score();
            account.score.update(new_score.adjust(&account.score.timezone));

            event.push(ScoreData {
                account_id,
                score: new_score.to_event(),
            });
        }

        emit(EventKind::RecordScore(event));
    }
}

trait ScoreConverter {
    /// Convert Score to a User's timezone
    fn adjust(&self, timezone: &Timezone) -> Chain;
    fn to_event(&self) -> Vec<(U32, UTC)>;
}

impl ScoreConverter for Vec<(Score, UTC)> {
    fn adjust(&self, timezone: &Timezone) -> Chain {
        self.iter().map(|score| (score.0, timezone.adjust(score.1))).collect()
    }

    fn to_event(&self) -> Vec<(U32, UTC)> {
        self.iter()
            .copied()
            .map(|(score, timestamp)| (U32(score.into()), timestamp))
            .collect()
    }
}
