use near_sdk::{env, near_bindgen, store::LookupMap, AccountId};
use sweat_jar_model::api::MigrationToClaimRemainder;

use crate::{jar::model_v1::ContractLegacy, Contract, ContractExt, StorageKey};

#[near_bindgen]
impl MigrationToClaimRemainder for Contract {
    #[init(ignore_state)]
    #[mutants::skip]
    fn migrate_state_to_claim_remainder() -> Self {
        let old_state: ContractLegacy = env::state_read().expect("failed");

        Contract {
            token_account_id: old_state.token_account_id,
            fee_account_id: old_state.fee_account_id,
            manager: old_state.manager,
            products: old_state.products,
            last_jar_id: old_state.last_jar_id,
            account_jars: LookupMap::new(StorageKey::AccountJarsV2),
            account_jars_v1: LookupMap::new(StorageKey::AccountJars),
        }
    }

    #[mutants::skip]
    fn migrate_accounts_to_claim_remainder(&mut self, accounts: Vec<AccountId>) {
        for account in accounts {
            self.migrate_account_jars_if_needed(account);
        }
    }
}

impl Contract {
    // TODO: remove after V2 migration
    #[mutants::skip]
    pub fn migrate_account_jars_if_needed(&mut self, account_id: AccountId) {
        let Some(jars) = self.account_jars_v1.remove(&account_id) else {
            return;
        };

        self.account_jars.insert(account_id, jars.into());
    }
}
