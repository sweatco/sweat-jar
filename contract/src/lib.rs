use std::{cell::RefCell, collections::HashMap};

use near_sdk::{
    collections::UnorderedMap, env, json_types::Base64VecU8, near, near_bindgen, store::LookupMap, AccountId,
    BorshStorageKey, PanicOnDefault,
};
use near_self_update_proc::SelfUpdate;
use sweat_jar_model::{api::InitApi, jar::JarId, ProductId};

use crate::{
    jar::{
        account::{v2::AccountV2, versioned::Account},
        model::AccountJarsLegacy,
    },
    migration::account_jars_non_versioned::AccountJarsNonVersioned,
    product::model::v2::ProductV2,
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
    pub products: UnorderedMap<ProductId, ProductV2>,

    /// The last jar ID. Is used as nonce in `get_ticket_hash` method.
    pub last_jar_id: JarId,

    pub accounts_v2: LookupMap<AccountId, AccountV2>,

    /// A lookup map that associates account IDs with sets of jars owned by each account.
    pub accounts: LookupMap<AccountId, Account>,

    pub account_jars_non_versioned: LookupMap<AccountId, AccountJarsNonVersioned>,
    pub account_jars_v1: LookupMap<AccountId, AccountJarsLegacy>,

    /// Cache to make access to products faster
    /// Is not stored in contract state so it should be always skipped by borsh
    #[borsh(skip)]
    pub products_cache: RefCell<HashMap<ProductId, ProductV2>>,
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
    /// Products migrated to step jars
    ProductsV2,
    /// Previous implementation of Score storage used on testnet. Is not used anymore.
    AccountScore,
    AccountsVersioned,
    AccountsV2,
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
            account_jars_non_versioned: LookupMap::new(StorageKey::AccountJarsV1),
            account_jars_v1: LookupMap::new(StorageKey::AccountJarsLegacy),
            last_jar_id: 0,
            accounts: LookupMap::new(StorageKey::AccountsVersioned),
            products_cache: HashMap::default().into(),
            accounts_v2: LookupMap::new(StorageKey::AccountsV2),
        }
    }
}
