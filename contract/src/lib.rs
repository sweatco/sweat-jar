use std::{cell::RefCell, collections::HashMap};

use near_plugins::{access_control, AccessControlRole, AccessControllable, Upgradable};
use near_sdk::{
    borsh::BorshDeserialize, collections::UnorderedMap, env, json_types::Base64VecU8, near, store::LookupMap,
    AccountId, BorshStorageKey, PanicOnDefault,
};
use sweat_jar_model::{
    api::{InitApi, RoleAssignments},
    data::{
        account::versioned::AccountVersioned,
        product::{Product, ProductId},
    },
    TokenAmount,
};

mod common;
mod doc;
mod feature;
mod migration;

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Roles for `near_plugins`' `AccessControllable`. `StagingManager`/`UpgradeManager`
/// are kept separate from the operational roles above them: code-deployment is a
/// materially more dangerous capability than unlocking jars or toggling feature
/// flags, so an account holding `Maintainer` must not automatically gain it.
#[derive(AccessControlRole, Copy, Clone)]
pub enum Roles {
    Oracle,
    ProductManager,
    FeeManager,
    Maintainer,
    StagingManager,
    UpgradeManager,
}

/// The `Contract` struct represents the state of the smart contract managing fungible token deposit jars.
///
/// The layout is identical in production and integration-test builds. The
/// integration-test time scale is persisted under its own raw storage key
/// (see `sweat_jar_model::time_scale`), the same pattern `near_plugins`'
/// `AccessControllable` uses for its `__acl` storage, so it needs no field here.
#[near(contract_state)]
#[derive(PanicOnDefault, Upgradable)]
#[access_control(role_type(Roles))]
#[upgradable(access_control_roles(
    code_stagers(Roles::StagingManager),
    code_deployers(Roles::UpgradeManager),
    duration_initializers(Roles::UpgradeManager),
    duration_update_stagers(Roles::UpgradeManager),
    duration_update_appliers(Roles::UpgradeManager),
))]
pub struct Contract {
    /// The account ID of the fungible token contract (NEP-141) that this jars contract interacts with.
    pub token_account_id: AccountId,

    /// The account ID where fees for applicable operations are directed.
    pub fee_account_id: AccountId,

    /// A collection of products, each representing terms for specific deposit jars.
    pub products: UnorderedMap<ProductId, Product>,

    /// A lookup map that associates account IDs with sets of jars owned by each account.
    pub accounts: LookupMap<AccountId, AccountVersioned>,

    /// Cache to make access to products faster.
    /// Is not stored in contract state (skipped by borsh).
    #[borsh(skip)]
    pub products_cache: RefCell<HashMap<ProductId, Product>>,

    pub fee_amount: TokenAmount,
    pub previous_version_account_id: AccountId,
}

#[near]
#[derive(BorshStorageKey)]
pub(crate) enum StorageKey {
    Products,
    Accounts,
}

#[near]
impl InitApi for Contract {
    #[init]
    #[private]
    fn init(
        token_account_id: AccountId,
        fee_account_id: AccountId,
        previous_version_account_id: AccountId,
        super_admin: AccountId,
        roles: RoleAssignments,
    ) -> Self {
        let mut contract = Self {
            token_account_id,
            fee_account_id,
            products: UnorderedMap::new(StorageKey::Products),
            products_cache: HashMap::default().into(),
            accounts: LookupMap::new(StorageKey::Accounts),
            fee_amount: 0,
            previous_version_account_id,
        };

        init_authority(&mut contract, super_admin, roles);

        contract
    }
}

/// Bootstraps the contract's own account as a temporary super-admin (required
/// because `acl_grant_role`/`acl_transfer_super_admin` both check
/// `env::predecessor_account_id()` for permission, not the account being
/// granted/named), grants every role in `roles`, then transfers super-admin
/// status to `super_admin`. The contract's own account retains no admin power
/// once this returns, unless `super_admin` is the contract's own account.
///
/// `acl_grant_role` (used by `grant_role_assignments`) only succeeds when the
/// predecessor already holds admin permission for the role being granted, and a
/// fresh `AccessControllable` storage has no admins yet. So the predecessor
/// (forced to equal `current_account_id` by `#[private]` at both call sites) is
/// bootstrapped as the first super-admin to perform the grants, then handed off
/// to the caller-supplied `super_admin` via `acl_transfer_super_admin`, which
/// adds `super_admin` and revokes the bootstrap account in one step (a no-op if
/// they're the same account). Shared by `InitApi::init` (fresh deploys) and
/// `migrate_access_control` (the already-deployed contract) so both entry
/// points behave identically here instead of duplicating this security-critical
/// sequence.
pub(crate) fn init_authority(contract: &mut Contract, super_admin: AccountId, roles: RoleAssignments) {
    contract.acl_init_super_admin(env::predecessor_account_id());
    grant_role_assignments(contract, roles);
    contract.acl_transfer_super_admin(super_admin);
}

/// Grants every account listed in `roles` its corresponding `AccessControllable`
/// role. Used internally by `init_authority`, which is itself shared by
/// `InitApi::init` (fresh deploys) and `migrate_access_control` (the
/// already-deployed contract), so both entry points assign roles identically
/// instead of duplicating this loop.
pub(crate) fn grant_role_assignments(contract: &mut Contract, roles: RoleAssignments) {
    for account_id in roles.oracle {
        contract.acl_grant_role("Oracle".to_string(), account_id);
    }
    for account_id in roles.product_manager {
        contract.acl_grant_role("ProductManager".to_string(), account_id);
    }
    for account_id in roles.fee_manager {
        contract.acl_grant_role("FeeManager".to_string(), account_id);
    }
    for account_id in roles.maintainer {
        contract.acl_grant_role("Maintainer".to_string(), account_id);
    }
    for account_id in roles.staging_manager {
        contract.acl_grant_role("StagingManager".to_string(), account_id);
    }
    for account_id in roles.upgrade_manager {
        contract.acl_grant_role("UpgradeManager".to_string(), account_id);
    }
}
