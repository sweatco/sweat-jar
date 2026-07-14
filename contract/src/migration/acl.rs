use std::cell::RefCell;

use near_sdk::{collections::UnorderedMap, env, near, store::LookupMap, AccountId, PanicOnDefault};
use sweat_jar_model::{jar::JarId, ProductId};

use crate::{
    jar::{account::versioned::Account, model::AccountJarsLegacy},
    migration::account_jars_non_versioned::AccountJarsNonVersioned,
    product::model::Product,
    Contract, ContractExt, MigrationState, RoleAssignments,
};

#[near]
#[derive(PanicOnDefault)]
pub struct ContractBeforeAcl {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub manager: AccountId,
    pub products: UnorderedMap<ProductId, Product>,
    pub last_jar_id: JarId,
    pub accounts: LookupMap<AccountId, Account>,
    pub account_jars_non_versioned: LookupMap<AccountId, AccountJarsNonVersioned>,
    pub account_jars_v1: LookupMap<AccountId, AccountJarsLegacy>,
    pub migration: MigrationState,
}

#[near]
#[mutants::skip]
impl Contract {
    #[private]
    #[init(ignore_state)]
    pub fn migrate_state_to_acl(super_admin: AccountId, roles: RoleAssignments) -> Self {
        let old_state: ContractBeforeAcl = env::state_read().expect("Failed to extract old contract state.");

        let mut contract = Self {
            token_account_id: old_state.token_account_id,
            fee_account_id: old_state.fee_account_id,
            products: old_state.products,
            last_jar_id: old_state.last_jar_id,
            accounts: old_state.accounts,
            account_jars_non_versioned: old_state.account_jars_non_versioned,
            account_jars_v1: old_state.account_jars_v1,
            products_cache: RefCell::default(),
            migration: old_state.migration,
        };

        contract.init_authority(super_admin, roles);

        contract
    }
}
