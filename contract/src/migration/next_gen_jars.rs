use std::collections::HashMap;

use near_sdk::{collections::UnorderedMap, env, near, near_bindgen, store::LookupMap, AccountId, PanicOnDefault};
use sweat_jar_model::{api::StateMigration, jar::JarId, product::Product, ProductId};

use crate::{
    jar::model::{AccountLegacyV1, AccountLegacyV2, AccountLegacyV3Wrapper},
    product::model::legacy::ProductLegacy,
    Archive, Contract, ContractExt, StorageKey,
};

#[near]
#[derive(PanicOnDefault)]
pub struct ContractBeforeMigration {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub manager: AccountId,
    pub products: UnorderedMap<ProductId, ProductLegacy>,
    pub last_jar_id: JarId,
    pub accounts_v3: LookupMap<AccountId, AccountLegacyV3Wrapper>,
    pub accounts_v2: LookupMap<AccountId, AccountLegacyV2>,
    pub accounts_v1: LookupMap<AccountId, AccountLegacyV1>,
}

#[near_bindgen]
impl StateMigration for Contract {
    #[init(ignore_state)]
    #[private]
    #[mutants::skip]
    fn migrate_state() -> Self {
        let mut old_state: ContractBeforeMigration = env::state_read().expect("Failed to extract old contract state.");

        let mut products: UnorderedMap<ProductId, Product> = UnorderedMap::new(StorageKey::Products);

        for (product_id, product) in &old_state.products {
            products.insert(&product_id, &product.into());
        }

        old_state.products.clear();

        Contract {
            token_account_id: old_state.token_account_id,
            fee_account_id: old_state.fee_account_id,
            manager: old_state.manager,
            products,
            accounts: LookupMap::new(StorageKey::Accounts),
            products_cache: HashMap::default().into(),
            fee_amount: 0,
            archive: Archive {
                accounts_v1: old_state.accounts_v1,
                accounts_v2: old_state.accounts_v2,
                accounts_v3: old_state.accounts_v3,
            },
        }
    }
}
