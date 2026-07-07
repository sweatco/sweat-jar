use std::cell::RefCell;

use near_plugins::AccessControllable;
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
    data::{
        account::versioned::AccountVersioned,
        product::{Product, ProductId},
    },
    TokenAmount,
};

use crate::{
    common::event::{emit, EventKind},
    Contract, ContractExt, StorageKey,
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
    /// storage, granting every role to the account that used to be `manager`.
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
    pub fn migrate_access_control() -> Self {
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

        contract.acl_init_super_admin(env::current_account_id());
        for role in ["Oracle", "ProductManager", "FeeManager", "Maintainer"] {
            contract.acl_grant_role(role.to_string(), old.manager.clone());
        }

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
