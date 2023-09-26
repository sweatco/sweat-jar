use std::ops::{Deref, DerefMut};

use ed25519_dalek::Signature;
use model::ProductId;
use near_sdk::{
    assert_one_yocto,
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::Base64VecU8,
    near_bindgen,
    store::{LookupMap, UnorderedMap},
    AccountId, BorshStorageKey, Gas, PanicOnDefault, Promise,
};
use near_self_update::SelfUpdate;
use product::model::{Apy, Product};

use crate::{
    assert::assert_ownership,
    jar::model::{Jar, JarId},
};

mod assert;
mod claim;
mod common;
mod event;
mod ft_interface;
mod ft_receiver;
mod internal;
mod jar;
mod migration;
mod penalty;
mod product;
mod tests;
mod withdraw;

// TODO: document all the numbers
// TODO: document gas amounts and how we got these numbers

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, SelfUpdate)]
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
}

#[derive(Default, BorshDeserialize, BorshSerialize)]
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

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Products,
    AccountJars,
}

#[near_bindgen]
impl Contract {
    #[init]
    #[private]
    #[must_use]
    pub fn init(token_account_id: AccountId, fee_account_id: AccountId, manager: AccountId) -> Self {
        Self {
            token_account_id,
            fee_account_id,
            manager,
            products: UnorderedMap::new(StorageKey::Products),
            account_jars: LookupMap::new(StorageKey::AccountJars),
            last_jar_id: 0,
        }
    }
}

pub(crate) trait JarsStorage {
    fn get_jar(&self, id: JarId) -> &Jar;
    fn get_jar_mut(&mut self, id: JarId) -> &mut Jar;
}

impl JarsStorage for Vec<Jar> {
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
