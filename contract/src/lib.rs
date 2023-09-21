use ed25519_dalek::Signature;
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
use product::model::{Apy, Product, ProductId};

use crate::{
    assert::assert_ownership,
    jar::model::{Jar, JarID},
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

    /// TODO: doc
    pub last_jar_id: JarID,

    /// A lookup map that associates account IDs with sets of jars owned by each account.
    pub account_jars: LookupMap<AccountId, Vec<Jar>>,

    /// TODO: document
    empty_jars: Vec<Jar>,
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
            empty_jars: vec![],
            last_jar_id: 0,
        }
    }
}

pub(crate) trait JarsStorage {
    fn get_jar(&self, id: JarID) -> &Jar;
    fn get_jar_mut(&mut self, id: JarID) -> &mut Jar;
}

impl JarsStorage for Vec<Jar> {
    fn get_jar(&self, id: JarID) -> &Jar {
        self.iter()
            .find(|jar| jar.id == id)
            .unwrap_or_else(|| env::panic_str(&format!("Jar with id: {id} doesn't exist")))
    }

    fn get_jar_mut(&mut self, id: JarID) -> &mut Jar {
        self.iter_mut()
            .find(|jar| jar.id == id)
            .unwrap_or_else(|| env::panic_str(&format!("Jar with id: {id} doesn't exist")))
    }
}
