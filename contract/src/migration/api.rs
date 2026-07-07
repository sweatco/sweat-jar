use std::cell::RefCell;

use near_sdk::{
    collections::UnorderedMap,
    env,
    json_types::Base64VecU8,
    near,
    store::{
        key::{Identity, ToKey},
        LookupMap,
    },
    AccountId, IntoStorageKey,
};
use sweat_jar_model::{
    api::RoleAssignments,
    data::{
        account::versioned::AccountVersioned,
        product::{Product, ProductId},
    },
    TokenAmount,
};

use crate::{
    common::event::{emit, EventKind},
    init_authority, Contract, ContractExt, StorageKey,
};

#[near]
impl Contract {
    pub fn migrate_products(&mut self, products: Vec<Product>) {
        self.assert_migrate_from_previous_version(&env::predecessor_account_id());

        let mut product_ids = Vec::new();

        for product in products {
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
    pub fn migrate_access_control(super_admin: AccountId, roles: RoleAssignments) -> Self {
        let old: OldContract = env::state_read().expect("failed to read old state");

        let mut contract = Self {
            token_account_id: old.token_account_id,
            fee_account_id: old.fee_account_id,
            products: old.products,
            accounts: old.accounts,
            products_cache: RefCell::default(),
            fee_amount: old.fee_amount,
            previous_version_account_id: old.previous_version_account_id,
            #[cfg(feature = "integration-test")]
            time_scale: 1.0,
        };

        // See `init_authority`'s doc comment (contract/src/lib.rs) for why this bootstrap-
        // then-transfer dance is needed and why it's shared with `InitApi::init`.
        init_authority(&mut contract, super_admin, roles);

        contract
    }
}

/// Mirrors the on-chain Borsh layout of `Contract` as it exists before this
/// migration runs (7 fields, including `manager`). Field order must match the
/// old, already-deployed struct exactly — Borsh deserializes by position, not
/// by name. `pub(crate)` (not private) so `migration::tests` can construct one
/// directly for the unit test in Step 1. Derives `BorshSerialize` too (not just
/// `BorshDeserialize`, which is all `migrate_access_control` itself needs) so
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

pub(crate) fn store_account_raw(account_id: AccountId, account_bytes: Base64VecU8) {
    let key = Identity::to_key(
        &StorageKey::Accounts.into_storage_key(),
        account_id.as_bytes(),
        &mut Vec::new(),
    );
    env::storage_write(&key, &account_bytes.0);
}
