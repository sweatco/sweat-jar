use near_sdk::{
    json_types::{I64, U128},
    near_bindgen, AccountId,
};
use sweat_jar_model::{
    api::ScoreApi,
    data::product::{Product, Terms},
    Score, Timezone, UTC,
};

use crate::{
    event::{emit, EventKind, ScoreData},
    score::Chain,
    Contract, ContractExt,
};

#[near_bindgen]
impl ScoreApi for Contract {
    fn record_score(&mut self, batch: Vec<(AccountId, Vec<(Score, UTC)>)>) {
        self.assert_manager();

        for (account_id, _) in &batch {
            self.assert_migrated(account_id);
        }

        let mut event = vec![];

        for (account_id, new_score) in batch {
            self.update_account_cache(
                &account_id,
                Some(|product: &Product| matches!(product.terms, Terms::ScoreBased(_))),
            );

            let account = self.get_account_mut(&account_id);
            account.score.try_reset_score();
            account.score.update(new_score.adjust(account.score.timezone));

            event.push(ScoreData {
                account_id,
                score: new_score,
            });
        }

        emit(EventKind::RecordScore(event));
    }

    fn get_timezone(&self, account_id: AccountId) -> Option<I64> {
        self.accounts
            .get(&account_id)
            .map(|account| I64(*account.score.timezone))
    }

    fn get_score(&self, account_id: AccountId) -> Option<U128> {
        let account = self.get_account(&account_id);

        Some(u128::from(account.score.active_score()).into())
    }
}

trait ScoreConverter {
    /// Convert Score to a User's timezone
    fn adjust(&self, timezone: Timezone) -> Chain;
}

impl ScoreConverter for Vec<(Score, UTC)> {
    fn adjust(&self, timezone: Timezone) -> Chain {
        self.iter().map(|score| (score.0, timezone.adjust(score.1))).collect()
    }
}
