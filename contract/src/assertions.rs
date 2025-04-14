use near_sdk::{require, AccountId};

use crate::Contract;

impl Contract {
    pub(crate) fn assert_migrate_from_previous_version(&self, account_id: &AccountId) {
        require!(account_id.clone() == self.previous_version_account_id, "Can migrate data only from previous version");
    }
}
