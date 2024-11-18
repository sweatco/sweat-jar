use std::collections::HashMap;

use near_sdk::{collections::UnorderedMap, env, near, near_bindgen, store::LookupMap, AccountId, PanicOnDefault};
use sweat_jar_model::{jar::JarId, ProductId};

use crate::{
    jar::model::AccountJarsLegacy,
    migration::account_jars_non_versioned::AccountJarsNonVersioned,
    product::model::{v1::Product as ProductLegacy, ProductV2},
    Contract, ContractExt, StorageKey,
};

#[near]
#[derive(PanicOnDefault)]
pub struct ContractLegacy {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub manager: AccountId,
    pub products: UnorderedMap<ProductId, ProductLegacy>,
    pub last_jar_id: JarId,
    pub account_jars: LookupMap<AccountId, AccountJarsNonVersioned>,
    pub account_jars_v1: LookupMap<AccountId, AccountJarsLegacy>,
}

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    #[private]
    #[mutants::skip]
    pub fn migrate() -> Self {
        let mut old_state: ContractLegacy = env::state_read().expect("Failed to extract old contract state.");

        let mut products: UnorderedMap<ProductId, ProductV2> = UnorderedMap::new(StorageKey::ProductsV2);

        for (product_id, product) in &old_state.products {
            products.insert(&product_id, &product.into());
        }

        old_state.products.clear();

        Contract {
            token_account_id: old_state.token_account_id,
            fee_account_id: old_state.fee_account_id,
            manager: old_state.manager,
            products,
            last_jar_id: old_state.last_jar_id,
            accounts: LookupMap::new(StorageKey::AccountsVersioned),
            account_jars_non_versioned: LookupMap::new(StorageKey::AccountJarsV1),
            account_jars_v1: LookupMap::new(StorageKey::AccountJarsLegacy),
            accounts_v2: LookupMap::new(StorageKey::AccountsV2),
            products_cache: HashMap::default().into(),
        }
    }
}
