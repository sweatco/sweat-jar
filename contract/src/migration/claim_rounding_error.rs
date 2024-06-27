#![allow(deprecated)]

use near_sdk::{near_bindgen, AccountId};
use sweat_jar_model::api::MigrationToClaimRemainder;

use crate::{Contract, ContractExt};

#[near_bindgen]
impl MigrationToClaimRemainder for Contract {
    #[mutants::skip]
    fn migrate_accounts_to_claim_remainder(&mut self, accounts: Vec<AccountId>) {
        for account in accounts {
            self.migrate_account_jars_if_needed(&account);
        }
    }
}

impl Contract {
    // TODO: remove after V2 migration
    #[mutants::skip]
    pub fn migrate_account_jars_if_needed(&mut self, account_id: &AccountId) {
        let Some(jars) = self.account_jars_v1.remove(account_id) else {
            return;
        };

        self.account_jars.insert(account_id.clone(), jars.into());
    }
}
