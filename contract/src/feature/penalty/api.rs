use near_sdk::{env, near, AccountId};
#[allow(deprecated)]
use sweat_jar_model::api::PenaltyApi;
use sweat_jar_model::data::account::{common::FeaturesAccess, features::Feature};

use crate::{
    common::event::{
        emit, BatchPenaltyData,
        EventKind::{ApplyPenalty, BatchApplyPenalty},
        PenaltyData,
    },
    Contract, ContractExt,
};

#[near]
#[allow(deprecated)]
impl PenaltyApi for Contract {
    #[allow(deprecated)]
    fn set_penalty(&mut self, account_id: AccountId, value: bool) {
        self.assert_manager();

        self.update_account_cache(&account_id, None);

        let account = self.get_account_mut(&account_id);
        account.set_feature_enabled(&Feature::IncreasedApy, !value);

        emit(ApplyPenalty(PenaltyData {
            account_id,
            is_applied: value,
            timestamp: env::block_timestamp_ms(),
        }));
    }

    #[allow(deprecated)]
    fn batch_set_penalty(&mut self, account_ids: Vec<AccountId>, value: bool) {
        self.assert_manager();

        for account_id in &account_ids {
            self.update_account_cache(account_id, None);

            let account = self.get_account_mut(account_id);
            account.set_feature_enabled(&Feature::IncreasedApy, !value);
        }

        emit(BatchApplyPenalty(BatchPenaltyData {
            account_ids,
            is_applied: value,
            timestamp: env::block_timestamp_ms(),
        }));
    }

    #[allow(deprecated)]
    fn is_penalty_applied(&self, account_id: AccountId) -> bool {
        !self.get_account(&account_id).is_feature_enabled(&Feature::IncreasedApy)
    }
}
