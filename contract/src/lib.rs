use ed25519_dalek::Signature;
use near_sdk::{
    assert_one_yocto,
    borsh::{self, maybestd::collections::HashSet, BorshDeserialize, BorshSerialize},
    env,
    json_types::Base64VecU8,
    near_bindgen,
    store::{LookupMap, UnorderedMap, Vector},
    AccountId, BorshStorageKey, Gas, PanicOnDefault, Promise,
};
use near_self_update::SelfUpdate;
use product::model::{Apy, Product, ProductId};

use crate::{
    assert::{assert_is_not_closed, assert_ownership},
    jar::model::{Jar, JarIndex, JarState},
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

#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct AccountJars {
    pub last_index: JarIndex,
    pub jars: HashSet<Jar>,
}

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

    /// A vector containing information about all deposit jars.
    pub jars: Vector<Jar>,

    /// A lookup map that associates account IDs with sets of jars owned by each account.
    pub account_jars: LookupMap<AccountId, AccountJars>,

    /// TODO: document
    pub empty_jars: HashSet<Jar>,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Products,
    Jars,
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
            jars: Vector::new(StorageKey::Jars),
            account_jars: LookupMap::new(StorageKey::AccountJars),
            empty_jars: Default::default(),
        }
    }
}
