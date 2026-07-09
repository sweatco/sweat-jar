use std::{cell::RefCell, collections::HashMap};

use near_plugins::{access_control, AccessControlRole, AccessControllable, Upgradable};
use near_sdk::{
    borsh::BorshDeserialize, collections::UnorderedMap, env, json_types::Base64VecU8, near, require, store::LookupMap,
    AccountId, BorshStorageKey, PanicOnDefault,
};
use strum::{EnumIter, IntoEnumIterator};
use sweat_jar_model::{
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
///
/// Must be defined in the same module as the `#[access_control]`-annotated
/// `Contract` struct: the `AccessControlRole` derive generates a private
/// `RoleFlags` type here that the `access_control` expansion refers to by
/// name. Test crates use this enum too (the contract builds as an `rlib` in
/// addition to the wasm `cdylib`) so role names have a single source of truth.
#[near(serializers = [json])]
#[derive(AccessControlRole, Copy, Clone, Debug, PartialEq, Eq, Hash, EnumIter)]
pub enum Roles {
    Oracle,
    ProductManager,
    FeeManager,
    Maintainer,
    StagingManager,
    UpgradeManager,
}

impl Roles {
    /// All role variants, without consumers needing to import strum's
    /// `IntoEnumIterator` trait.
    pub fn all() -> impl Iterator<Item = Self> {
        Self::iter()
    }
}

/// Initial holders for each `near_plugins` `AccessControllable` role, passed
/// explicitly to `init`/`migrate` rather than defaulting to any particular
/// account. Each role accepts multiple initial holders; granting a role to
/// further accounts later is still possible via the standard `acl_grant_role`
/// method, unaffected by this map. Roles absent from the map get no initial
/// holders.
pub type RoleAssignments = HashMap<Roles, Vec<AccountId>>;

/// Convenience for the common case: one account holding every role.
pub fn all_roles_to(account_id: &AccountId) -> RoleAssignments {
    Roles::all().map(|role| (role, vec![account_id.clone()])).collect()
}

pub trait InitApi {
    fn init(
        token_account_id: AccountId,
        fee_account_id: AccountId,
        previous_version_account_id: AccountId,
        super_admin: AccountId,
        roles: RoleAssignments,
    ) -> Self;
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

        contract.init_authority(super_admin, roles);

        contract
    }
}

// Deliberately NOT `#[near]`-annotated: these are internal helpers, not
// contract methods, and the `#[near]` macro would force them `pub`.
impl Contract {
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
    /// `migrate` (the already-deployed contract) so both entry points behave
    /// identically here instead of duplicating this security-critical sequence.
    pub(crate) fn init_authority(&mut self, super_admin: AccountId, roles: RoleAssignments) {
        // `acl_init_super_admin` returns `false` iff a super admin already
        // exists — i.e. this ACL storage was bootstrapped before. Failing loudly
        // here (rather than continuing with silently no-op'd grants) also makes
        // any accidental second run of `migrate` a deterministic revert.
        require!(
            self.acl_init_super_admin(env::predecessor_account_id()),
            "ACL bootstrap failed: super admin is already initialized"
        );
        self.grant_role_assignments(roles);
        // `None` means the predecessor is not a super admin — must be impossible
        // right after the bootstrap above, but a silent failure here would leave
        // the contract without its intended super admin, so verify.
        require!(
            self.acl_transfer_super_admin(super_admin).is_some(),
            "ACL bootstrap failed: could not transfer super admin"
        );
    }

    /// Grants every account listed in `roles` its corresponding `AccessControllable`
    /// role. Used internally by `init_authority`, which is itself shared by
    /// `InitApi::init` (fresh deploys) and `migrate` (the already-deployed
    /// contract), so both entry points assign roles identically instead of
    /// duplicating this loop.
    pub(crate) fn grant_role_assignments(&mut self, roles: RoleAssignments) {
        for (role, account_ids) in roles {
            for account_id in account_ids {
                // `None` means the predecessor lacks admin permission for this
                // role; a grant silently not happening must fail the whole
                // bootstrap instead of leaving a partially-provisioned ACL.
                // (`Some(false)` — account already held the role — is fine.)
                require!(
                    self.acl_grant_role(role.into(), account_id).is_some(),
                    "ACL bootstrap failed: could not grant role"
                );
            }
        }
    }
}
