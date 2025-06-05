use near_sdk::{
    env,
    json_types::Base64VecU8,
    near,
    store::key::{Identity, ToKey},
    AccountId, IntoStorageKey,
};
use sweat_jar_model::data::product::Product;

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
}

pub(crate) fn store_account_raw(account_id: AccountId, account_bytes: Base64VecU8) {
    let key = Identity::to_key(
        &StorageKey::Accounts.into_storage_key(),
        account_id.as_bytes(),
        &mut Vec::new(),
    );
    env::storage_write(&key, &account_bytes.0);
}
