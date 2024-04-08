use near_sdk::{
    env, near, near_bindgen,
    store::{LookupMap, UnorderedMap},
    AccountId, PanicOnDefault,
};
use sweat_jar_model::{api::MigrationToClaimRemainder, jar::JarId, ProductId};

use crate::{jar::model::AccountJarsMapLegacy, product::model::Product, Contract, ContractExt, StorageKey};

#[near]
#[derive(PanicOnDefault)]
pub struct ContractLegacy {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub manager: AccountId,
    pub products: UnorderedMap<ProductId, Product>,
    pub last_jar_id: JarId,
    pub account_jars: AccountJarsMapLegacy,
}

#[near_bindgen]
impl MigrationToClaimRemainder for Contract {
    #[private]
    #[init(ignore_state)]
    #[mutants::skip]
    fn migrate_state_to_claim_remainder() -> Self {
        let old_state: ContractLegacy = env::state_read().expect("Failed to extract old contract state.");

        Contract {
            token_account_id: old_state.token_account_id,
            fee_account_id: old_state.fee_account_id,
            manager: old_state.manager,
            products: old_state.products,
            last_jar_id: old_state.last_jar_id,
            account_jars: LookupMap::new(StorageKey::AccountJarsV1),
            account_jars_v1: LookupMap::new(StorageKey::AccountJarsLegacy),
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
