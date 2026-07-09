use std::cell::RefCell;

use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{
    assert_one_yocto,
    borsh::BorshDeserialize,
    collections::UnorderedMap,
    env,
    json_types::Base64VecU8,
    near, require,
    store::{
        key::{Identity, ToKey},
        LookupMap,
    },
    AccountId, IntoStorageKey,
};
use sweat_jar_model::{
    data::{
        account::versioned::AccountVersioned,
        product::{Product, ProductAssertions, ProductId},
    },
    TokenAmount,
};

use crate::{
    common::event::{emit, EventKind},
    Contract, ContractExt, RoleAssignments, Roles, StorageKey,
};

#[near]
impl Contract {
    /// One-shot: retires the `previous_version_account_id` privilege so
    /// `migrate_products`/`FtMessage::Migrate` can never be called again,
    /// bounding the blast radius of a future compromise of that account.
    /// Idempotent — calling more than once is a harmless no-op, since
    /// `"system"` is a reserved NEAR account no signed transaction can ever
    /// have as its `predecessor_account_id`.
    /// Reversible via `enable_migration`.
    #[access_control_any(roles(Roles::Maintainer))]
    #[payable]
    pub fn disable_migration(&mut self) {
        assert_one_yocto();
        self.previous_version_account_id = "system".parse().unwrap();
        emit(EventKind::MigrationDisabled);
    }

    /// Symmetric to `disable_migration`: (re-)points migration acceptance at
    /// `previous_version_account_id`, so `migrate_products`/`FtMessage::Migrate`
    /// accept calls from that account again.
    #[access_control_any(roles(Roles::Maintainer))]
    #[payable]
    pub fn enable_migration(&mut self, previous_version_account_id: AccountId) {
        assert_one_yocto();
        self.previous_version_account_id = previous_version_account_id.clone();
        emit(EventKind::MigrationEnabled(previous_version_account_id));
    }

    pub fn migrate_products(&mut self, products: Vec<Product>) {
        self.assert_migrate_from_previous_version(&env::predecessor_account_id());

        let mut product_ids = Vec::new();

        for product in products {
            require!(self.products.get(&product.id).is_none(), "Product already exists");
            product.assert_score_based_product_is_protected();
            product.assert_fee_amount();
            product.assert_cap_order();
            product.assert_udecimal_exponents_in_range();

            self.products.insert(&product.id, &product);
            product_ids.push(product.id);
        }

        emit(EventKind::MigrateProducts(product_ids));
    }

    /// One-shot migration for the already-deployed contract: drops the
    /// `manager` field from the Borsh layout and bootstraps `AccessControllable`
    /// storage. `super_admin`/`roles` are explicit, required arguments — this
    /// method does not fall back to `old.manager` for any role. `old.manager`
    /// remains part of `OldContract` (required to correctly parse the old
    /// Borsh layout) but is intentionally never read here: whoever runs this
    /// migration decides explicitly who holds which role, even if that means
    /// passing the same account that used to be `manager` into several
    /// `RoleAssignments` fields.
    /// No `&self`/`&mut self` param — near-sdk must not try to auto-deserialize
    /// the *new* struct shape from the *old* on-chain bytes before this body
    /// runs; `env::state_read` does that manually against `OldContract` instead.
    /// `OldContract` and this method are one-shot: delete them in a follow-up
    /// cleanup commit once this has actually run against the live contract.
    ///
    /// `#[init(ignore_state)]` is required (not just `#[private]`): `near-sdk`
    /// 5.x forbids returning `Self` from a plain `Call`/`View` method, and a
    /// bare `#[init]` would refuse to run because state already exists on the
    /// already-deployed contract this migration targets.
    #[init(ignore_state)]
    #[private]
    pub fn migrate(super_admin: AccountId, roles: RoleAssignments) -> Self {
        let old: OldContract = env::state_read().expect("failed to read old state");

        let mut contract = Self {
            token_account_id: old.token_account_id,
            fee_account_id: old.fee_account_id,
            products: old.products,
            accounts: old.accounts,
            products_cache: RefCell::default(),
            fee_amount: old.fee_amount,
            previous_version_account_id: old.previous_version_account_id,
        };

        // See `init_authority`'s doc comment (contract/src/lib.rs) for why this bootstrap-
        // then-transfer dance is needed and why it's shared with `InitApi::init`.
        contract.init_authority(super_admin, roles);

        contract
    }
}

/// Mirrors the on-chain Borsh layout of `Contract` as it exists before this
/// migration runs (7 fields, including `manager`). Field order must match the
/// old, already-deployed struct exactly — Borsh deserializes by position, not
/// by name. `pub(crate)` (not private) so `migration::tests` can construct one
/// directly for the unit test in Step 1. Derives `BorshSerialize` too (not just
/// `BorshDeserialize`, which is all `migrate` itself needs) so
/// that test can write old-shaped bytes into storage via `env::state_write`.
#[near(serializers=[borsh])]
pub(crate) struct OldContract {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub manager: AccountId,
    pub products: UnorderedMap<ProductId, Product>,
    pub accounts: LookupMap<AccountId, AccountVersioned>,
    pub fee_amount: TokenAmount,
    pub previous_version_account_id: AccountId,
}

fn account_storage_key(account_id: &AccountId) -> Vec<u8> {
    Identity::to_key(
        &StorageKey::Accounts.into_storage_key(),
        account_id.as_bytes(),
        &mut Vec::new(),
    )
    .clone()
}

pub(crate) fn store_account_raw(account_id: AccountId, account_bytes: Base64VecU8) {
    env::storage_write(&account_storage_key(&account_id), &account_bytes.0);
}

/// True if `account_id` has no account state yet, or has one that's still
/// entirely default. Reads storage directly via `env::storage_read` rather
/// than through `Contract::accounts` (a cached `LookupMap`) — deliberately,
/// since a normal typed read here would cache a `None` for a not-yet-existing
/// account, and that stale cache entry would then hide `store_account_raw`'s
/// subsequent raw write from every read for the rest of this execution (the
/// cache has no way to know storage changed underneath it).
pub(crate) fn is_new_or_empty_account(account_id: &AccountId) -> bool {
    match env::storage_read(&account_storage_key(account_id)) {
        None => true,
        Some(bytes) => {
            let versioned = AccountVersioned::try_from_slice(&bytes)
                .unwrap_or_else(|_| near_sdk::env::panic_str("Failed to deserialize existing account"));
            versioned.is_empty()
        }
    }
}
