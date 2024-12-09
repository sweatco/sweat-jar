use std::{cell::RefCell, collections::HashMap, ops::Deref};

use jar::model::JarVersionedLegacy;
use near_sdk::{
    collections::UnorderedMap, env, json_types::Base64VecU8, near, near_bindgen, store::LookupMap, AccountId,
    BorshStorageKey, PanicOnDefault,
};
use near_self_update_proc::SelfUpdate;
use sweat_jar_model::{api::InitApi, ProductId};

use crate::{
    jar::{
        account::versioned::AccountVersioned,
        model::{AccountLegacyV1, AccountLegacyV2, AccountLegacyV3, AccountLegacyV3Wrapper},
    },
    product::model::v1::Product,
};

mod assert;
mod claim;
mod common;
mod event;
mod ft_interface;
mod ft_receiver;
mod integration_test;
mod internal;
mod jar;
mod migration;
mod penalty;
mod product;
mod restake;
mod score;
mod test_utils;
mod tests;
mod withdraw;

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[near(contract_state)]
#[derive(PanicOnDefault, SelfUpdate)]
/// The `Contract` struct represents the state of the smart contract managing fungible token deposit jars.
pub struct Contract {
    /// The account ID of the fungible token contract (NEP-141) that this jars contract interacts with.
    pub token_account_id: AccountId,

    /// The account ID where fees for applicable operations are directed.
    pub fee_account_id: AccountId,

    /// The account ID authorized to perform sensitive operations on the contract.
    pub manager: AccountId,

    /// A collection of products, each representing terms for specific deposit jars.
    pub products: UnorderedMap<ProductId, Product>,

    /// A lookup map that associates account IDs with sets of jars owned by each account.
    pub accounts: LookupMap<AccountId, AccountVersioned>,

    /// Cache to make access to products faster
    /// Is not stored in contract state so it should be always skipped by borsh
    #[borsh(skip)]
    pub products_cache: RefCell<HashMap<ProductId, Product>>,

    pub archive: Archive,
}

#[near]
#[derive(BorshStorageKey)]
pub(crate) enum StorageKey {
    _ProductsLegacyV1,
    AccountsLegacyV1,
    /// Jars with claim remainder
    AccountsLegacyV2,
    /// Products migrated to near_sdk 5
    _ProductsLegacyV2,
    /// Score supporting products
    ProductsLegacyV2,
    AccountsLegacyV3,
    Products,
    Accounts,
}

#[near_bindgen]
impl InitApi for Contract {
    #[init]
    #[private]
    fn init(token_account_id: AccountId, fee_account_id: AccountId, manager: AccountId) -> Self {
        Self {
            token_account_id,
            fee_account_id,
            manager,
            products: UnorderedMap::new(StorageKey::Products),
            products_cache: HashMap::default().into(),
            accounts: LookupMap::new(StorageKey::Accounts),
            archive: Archive {
                accounts_v1: LookupMap::new(StorageKey::AccountsLegacyV1),
                accounts_v2: LookupMap::new(StorageKey::AccountsLegacyV2),
                accounts_v3: LookupMap::new(StorageKey::AccountsLegacyV3),
            },
        }
    }
}

#[near]
pub struct Archive {
    pub accounts_v1: LookupMap<AccountId, AccountLegacyV1>,
    pub accounts_v2: LookupMap<AccountId, AccountLegacyV2>,
    pub accounts_v3: LookupMap<AccountId, AccountLegacyV3Wrapper>,
}

impl Archive {
    fn contains_account(&self, account_id: &AccountId) -> bool {
        self.accounts_v1.contains_key(account_id) || self.accounts_v2.contains_key(account_id)
    }

    fn get_account(&self, account_id: &AccountId) -> Option<AccountLegacyV3> {
        if let Some(account) = self.accounts_v3.get(account_id).cloned() {
            return Some(account.deref().clone());
        }

        if let Some(account) = self.accounts_v2.get(account_id) {
            return Some(account.clone().into());
        }

        if let Some(account) = self.accounts_v1.get(account_id) {
            return Some(account.clone().into());
        }

        None
    }

    fn get_jars(&self, account_id: &AccountId) -> Option<Vec<JarVersionedLegacy>> {
        self.get_account(account_id).map(|account| account.jars)
    }
}
