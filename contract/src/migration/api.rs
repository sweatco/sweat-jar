use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{
    assert_one_yocto,
    borsh::BorshDeserialize,
    env,
    json_types::Base64VecU8,
    near, require,
    store::key::{Identity, ToKey},
    AccountId, IntoStorageKey,
};
use sweat_jar_model::data::{
    account::versioned::AccountVersioned,
    product::{Product, ProductAssertions},
};

use crate::{
    common::event::{emit, EventKind},
    Contract, ContractExt, Roles, StorageKey,
};

#[near]
impl Contract {
    /// Turns migration acceptance off by pointing `previous_version_account_id`
    /// at `"system"` — a reserved account no signed transaction can have as its
    /// predecessor — so `migrate_products`/`FtMessage::Migrate` reject every
    /// caller until `enable_migration` turns it back on. Keep it off once
    /// migration is complete.
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
}

fn account_storage_key(account_id: &AccountId) -> Vec<u8> {
    Identity::to_key(
        &StorageKey::Accounts.into_storage_key(),
        account_id.as_bytes(),
        &mut Vec::new(),
    )
}

pub(crate) fn store_account_raw(account_id: AccountId, account_bytes: Base64VecU8) {
    env::storage_write(&account_storage_key(&account_id), &account_bytes.0);
}

/// True if `account_id` has no account state yet, or one that's still entirely
/// default. Deliberately reads raw storage, not `Contract::accounts`: the
/// `LookupMap` would cache the `None` and then hide `store_account_raw`'s
/// subsequent raw write from every typed read in this execution.
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
