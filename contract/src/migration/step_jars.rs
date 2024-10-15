use near_sdk::{collections::UnorderedMap, near, store::LookupMap, AccountId, PanicOnDefault};
use sweat_jar_model::{jar::JarId, ProductId};

use crate::{
    jar::model::AccountJarsLegacy,
    migration::account_jars_non_versioned::AccountJarsNonVersioned,
    product::model::v1::{Apy, Cap, Terms, WithdrawalFee}
    ,
};

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug)]
pub struct ProductBeforeStepJars {
    pub id: ProductId,
    pub apy: Apy,
    pub cap: Cap,
    pub terms: Terms,
    pub withdrawal_fee: Option<WithdrawalFee>,
    pub public_key: Option<Vec<u8>>,
    pub is_enabled: bool,
}

#[near]
#[derive(PanicOnDefault)]
pub struct ContractBeforeStepJars {
    pub token_account_id: AccountId,
    pub fee_account_id: AccountId,
    pub manager: AccountId,
    pub products: UnorderedMap<ProductId, ProductBeforeStepJars>,
    pub last_jar_id: JarId,
    pub account_jars: LookupMap<AccountId, AccountJarsNonVersioned>,
    pub account_jars_v1: LookupMap<AccountId, AccountJarsLegacy>,
}

// TODO: this migration will be outdated at the moment of release
// #[near_bindgen]
// impl MigrationToStepJars for Contract {
//     #[private]
//     #[init(ignore_state)]
//     #[mutants::skip]
//     fn migrate_state_to_step_jars() -> Self {
//         let mut old_state: ContractBeforeStepJars = env::state_read().expect("Failed to extract old contract state.");
//
//         let mut products: UnorderedMap<ProductId, Product> = UnorderedMap::new(StorageKey::ProductsV2);
//
//         for (product_id, product) in &old_state.products {
//             products.insert(
//                 &product_id,
//                 &Product {
//                     id: product.id,
//                     apy: product.apy,
//                     cap: product.cap,
//                     terms: product.terms,
//                     withdrawal_fee: product.withdrawal_fee,
//                     public_key: product.public_key,
//                     is_enabled: product.is_enabled,
//                     score_cap: 0,
//                 },
//             );
//         }
//
//         old_state.products.clear();
//
//         Contract {
//             token_account_id: old_state.token_account_id,
//             fee_account_id: old_state.fee_account_id,
//             manager: old_state.manager,
//             products,
//             last_jar_id: old_state.last_jar_id,
//             accounts: LookupMap::new(StorageKey::AccountsVersioned),
//             account_jars_non_versioned: LookupMap::new(StorageKey::AccountJarsV1),
//             account_jars_v1: LookupMap::new(StorageKey::AccountJarsLegacy),
//             products_cache: HashMap::default().into(),
//         }
//     }
// }
