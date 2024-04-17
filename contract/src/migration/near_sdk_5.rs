#![allow(deprecated)]

use near_sdk::{
    env, near, near_bindgen,
    store::{LookupMap, UnorderedMap},
    AccountId, PanicOnDefault,
};
use sweat_jar_model::{api::MigratonToNearSdk5, jar::JarId, ProductId};

use crate::{jar::model::AccountJarsLegacy, product::model::Product, AccountJars, Contract, ContractExt, StorageKey};

#[near]
#[derive(PanicOnDefault)]
pub struct ContractBeforeNearSdk5 {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub manager: AccountId,
    pub products: UnorderedMap<ProductId, Product>,
    pub last_jar_id: JarId,
    pub account_jars: LookupMap<AccountId, AccountJars>,
    pub account_jars_v1: LookupMap<AccountId, AccountJarsLegacy>,
}

#[near_bindgen]
impl MigratonToNearSdk5 for Contract {
    #[private]
    #[init(ignore_state)]
    #[mutants::skip]
    fn migrate_state_to_near_sdk_5() -> Self {
        let old_state: ContractBeforeNearSdk5 = env::state_read().expect("Failed to extract old contract state.");

        let mut products: near_sdk::collections::UnorderedMap<ProductId, Product> =
            near_sdk::collections::UnorderedMap::new(StorageKey::ProductsV1);

        for (product_id, product) in &old_state.products {
            products.insert(product_id, product);
        }

        Contract {
            token_account_id: old_state.token_account_id,
            fee_account_id: old_state.fee_account_id,
            manager: old_state.manager,
            products,
            last_jar_id: old_state.last_jar_id,
            account_jars: LookupMap::new(StorageKey::AccountJarsV1),
            account_jars_v1: LookupMap::new(StorageKey::AccountJarsLegacy),
        }
    }
}
