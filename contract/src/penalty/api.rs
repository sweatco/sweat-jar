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

        self.migrate_account_if_needed(&account_id);

        let account = self.get_account_mut(&account_id);
        account.is_penalty_applied = value;
        self.update_account_cache(account);

        emit(ApplyPenalty(PenaltyData {
            account_id,
            is_applied: value,
            timestamp: env::block_timestamp_ms(),
        }));
    }

    fn batch_set_penalty(&mut self, account_ids: Vec<AccountId>, value: bool) {
        self.assert_manager();

        for account_id in account_ids.iter() {
            self.migrate_account_if_needed(&account_id);

            let account = self.get_account_mut(account_id);
            account.is_penalty_applied = value;
            self.update_account_cache(account);
        }

        emit(BatchApplyPenalty(BatchPenaltyData {
            account_ids,
            is_applied: value,
            timestamp: env::block_timestamp_ms(),
        }));
    }
}
