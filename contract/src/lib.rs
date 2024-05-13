use std::ops::{Deref, DerefMut};

use ed25519_dalek::Signature;
use near_sdk::{
    collections::UnorderedMap, env, json_types::Base64VecU8, near, near_bindgen, store::LookupMap, AccountId,
    BorshStorageKey, PanicOnDefault,
};
use near_self_update_proc::SelfUpdate;
use product::model::{Apy, Product};
use sweat_jar_model::{api::InitApi, jar::JarId, ProductId};

use crate::jar::model::{AccountJarsLegacy, Jar};

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
    pub account_jars: LookupMap<AccountId, AccountJars>,

    pub account_jars_v1: LookupMap<AccountId, AccountJarsLegacy>,
}

#[near]
#[derive(Default)]
pub struct AccountJars {
    /// The last jar ID. Is used as nonce in `get_ticket_hash` method.
    pub last_id: JarId,
    pub jars: Vec<Jar>,
}

impl Deref for AccountJars {
    type Target = Vec<Jar>;

    fn deref(&self) -> &Self::Target {
        &self.jars
    }
}

impl DerefMut for AccountJars {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.jars
    }
}

#[near]
#[derive(BorshStorageKey)]
pub(crate) enum StorageKey {
    ProductsLegacy,
    AccountJarsLegacy,
    /// Jars with claim remainder
    AccountJarsV1,
    /// Products migrated to near_sdk 5
    ProductsV1,
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
            products: UnorderedMap::new(StorageKey::ProductsV1),
            account_jars: LookupMap::new(StorageKey::AccountJarsV1),
            account_jars_v1: LookupMap::new(StorageKey::AccountJarsLegacy),
            last_jar_id: 0,
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
