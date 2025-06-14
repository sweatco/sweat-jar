use std::{cell::RefCell, collections::HashMap};

use near_sdk::{
    collections::UnorderedMap,
    env,
    json_types::Base64VecU8,
    near, near_bindgen,
    store::{LookupMap, LookupSet},
    AccountId, BorshStorageKey, PanicOnDefault,
};
use near_self_update_proc::SelfUpdate;
use product::model::{Apy, Product};
use sweat_jar_model::{api::InitApi, jar::JarId, ProductId};

use crate::{
    jar::{
        account::versioned::Account,
        model::{AccountJarsLegacy, Jar},
    },
    migration::account_jars_non_versioned::AccountJarsNonVersioned,
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
mod score;
mod test_builder;
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

    /// The last jar ID. Is used as nonce in `get_ticket_hash` method.
    pub last_jar_id: JarId,

    /// A lookup map that associates account IDs with sets of jars owned by each account.
    pub accounts: LookupMap<AccountId, Account>,

    pub account_jars_non_versioned: LookupMap<AccountId, AccountJarsNonVersioned>,
    pub account_jars_v1: LookupMap<AccountId, AccountJarsLegacy>,

    /// Cache to make access to products faster
    /// Is not stored in contract state so it should be always skipped by borsh
    #[borsh(skip)]
    pub products_cache: RefCell<HashMap<ProductId, Product>>,

    pub migration: MigrationState,
}

#[near]
pub struct MigrationState {
    pub new_version_account_id: AccountId,
    pub migrating_accounts: LookupSet<AccountId>,
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
    /// Products migrated to step jars
    Products,
    Accounts,
    _SkippedKey, // This was used in one of the migrations, but is not needed anymore
    Migration,
}

#[near_bindgen]
impl InitApi for Contract {
    #[init]
    #[private]
    fn init(
        token_account_id: AccountId,
        fee_account_id: AccountId,
        manager: AccountId,
        new_version_account_id: AccountId,
    ) -> Self {
        Self {
            token_account_id,
            fee_account_id,
            manager,
            products: UnorderedMap::new(StorageKey::_ProductsLegacyV2),
            account_jars_non_versioned: LookupMap::new(StorageKey::AccountsLegacyV2),
            account_jars_v1: LookupMap::new(StorageKey::AccountsLegacyV1),
            last_jar_id: 0,
            accounts: LookupMap::new(StorageKey::Accounts),
            products_cache: HashMap::default().into(),
            migration: MigrationState {
                new_version_account_id,
                migrating_accounts: LookupSet::new(StorageKey::Migration),
            },
        }
    }
}

pub(crate) trait JarsStorage<J> {
    fn get_jar(&self, id: JarId) -> &J;
    fn get_jar_mut(&mut self, id: JarId) -> &mut J;
}

impl JarsStorage<Jar> for Vec<Jar> {
    fn get_jar(&self, id: JarId) -> &Jar {
        self.iter()
            .find(|jar| jar.id == id)
            .unwrap_or_else(|| env::panic_str(&format!("Jar with id: {id} doesn't exist")))
    }

    fn get_jar_mut(&mut self, id: JarId) -> &mut Jar {
        self.iter_mut()
            .find(|jar| jar.id == id)
            .unwrap_or_else(|| env::panic_str(&format!("Jar with id: {id} doesn't exist")))
    }
}
