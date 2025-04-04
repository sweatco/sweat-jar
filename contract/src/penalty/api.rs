use near_sdk::{env, near_bindgen, AccountId};
use sweat_jar_model::api::PenaltyApi;

use crate::{
    event::{
        emit, BatchPenaltyData,
        EventKind::{ApplyPenalty, BatchApplyPenalty},
        PenaltyData,
    },
    Contract, ContractExt,
};

#[near_bindgen]
impl PenaltyApi for Contract {
    fn set_penalty(&mut self, account_id: AccountId, value: bool) {
        self.assert_manager();

        self.assert_migrated(&account_id);
        self.update_account_cache(&account_id, None);

        let account = self.get_account_mut(&account_id);
        account.is_penalty_applied = value;

        emit(ApplyPenalty(PenaltyData {
            account_id,
            is_applied: value,
            timestamp: env::block_timestamp_ms(),
        }));
    }

    fn batch_set_penalty(&mut self, account_ids: Vec<AccountId>, value: bool) {
        self.assert_manager();

        for account_id in &account_ids {
            self.assert_migrated(account_id);
            self.update_account_cache(account_id, None);

            let account = self.get_account_mut(account_id);
            account.is_penalty_applied = value;
        }

        emit(BatchApplyPenalty(BatchPenaltyData {
            account_ids,
            is_applied: value,
            timestamp: env::block_timestamp_ms(),
        }));
    }

    fn is_penalty_applied(&self, account_id: AccountId) -> bool {
        self.get_account(&account_id).is_penalty_applied
    }
}
