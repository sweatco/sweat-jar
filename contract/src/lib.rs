use std::{cell::RefCell, collections::HashMap};

use near_sdk::{
    collections::UnorderedMap, env, json_types::Base64VecU8, near, near_bindgen, store::LookupMap, AccountId,
    BorshStorageKey, PanicOnDefault,
};
use near_self_update_proc::SelfUpdate;
use sweat_jar_model::{
    api::InitApi,
    data::{
        account::versioned::AccountVersioned,
        product::{Product, ProductId},
    },
    TokenAmount,
};

#[cfg(feature = "integration-test")]
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

mod common;
mod doc;
mod feature;
mod migration;

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The `Contract` struct represents the state of the smart contract managing fungible token deposit jars.
#[cfg(not(feature = "integration-test"))]
#[near(contract_state)]
#[derive(PanicOnDefault, SelfUpdate)]
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

    /// Cache to make access to products faster.
    /// Is not stored in contract state (skipped by borsh).
    #[borsh(skip)]
    pub products_cache: RefCell<HashMap<ProductId, Product>>,

    pub fee_amount: TokenAmount,
    pub previous_version_account_id: AccountId,
}

/// The `Contract` struct represents the state of the smart contract managing fungible token deposit jars.
/// Integration test version with custom BorshSerialize/Deserialize that syncs time_scale to thread-local storage.
#[cfg(feature = "integration-test")]
#[near(contract_state, serializers = [])]
#[derive(PanicOnDefault, SelfUpdate)]
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

    /// Cache to make access to products faster.
    /// Is not stored in contract state (not serialized in custom BorshSerialize impl).
    pub products_cache: RefCell<HashMap<ProductId, Product>>,

    pub fee_amount: TokenAmount,
    pub previous_version_account_id: AccountId,
    pub boosters: Boosters,

    /// Time scale for integration tests, stored in blockchain state.
    /// This value is persisted and automatically synced to global thread-local storage on deserialization.
    /// The actual time scale is accessed in code via ms_in_day()/ms_in_year() functions.
    ///
    /// Examples:
    /// - `1.0` - Normal time (1 day = 24 hours)
    /// - `1.0/24.0` - Accelerated 24x (1 day = 1 hour)
    /// - `1.0/365.0` - Accelerated 365x (1 year = 1 day)
    pub time_scale: f64,
}

#[cfg(feature = "integration-test")]
impl BorshSerialize for Contract {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.token_account_id.serialize(writer)?;
        self.fee_account_id.serialize(writer)?;
        self.manager.serialize(writer)?;
        self.products.serialize(writer)?;
        self.accounts.serialize(writer)?;
        self.fee_amount.serialize(writer)?;
        self.previous_version_account_id.serialize(writer)?;
        self.boosters.serialize(writer)?;
        self.time_scale.serialize(writer)?;
        Ok(())
    }
}

#[cfg(feature = "integration-test")]
impl BorshDeserialize for Contract {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let token_account_id = AccountId::deserialize_reader(reader)?;
        let fee_account_id = AccountId::deserialize_reader(reader)?;
        let manager = AccountId::deserialize_reader(reader)?;
        let products = UnorderedMap::deserialize_reader(reader)?;
        let accounts = LookupMap::deserialize_reader(reader)?;
        let fee_amount = TokenAmount::deserialize_reader(reader)?;
        let previous_version_account_id = AccountId::deserialize_reader(reader)?;
        let boosters = Boosters::deserialize_reader(reader)?;
        let time_scale = f64::deserialize_reader(reader)?;

        // Sync time scale to global thread-local storage automatically on deserialization
        sweat_jar_model::set_global_time_scale(time_scale);

        Ok(Self {
            token_account_id,
            fee_account_id,
            manager,
            products,
            accounts,
            products_cache: RefCell::new(HashMap::new()),
            fee_amount,
            previous_version_account_id,
            boosters,
            time_scale,
        })
    }
}

#[near]
#[derive(BorshStorageKey)]
pub(crate) enum StorageKey {
    Products,
    Accounts,
    BoostersIndex,
    BoostersItems,
}

#[near]
impl InitApi for Contract {
    #[init]
    #[private]
    fn init(
        token_account_id: AccountId,
        fee_account_id: AccountId,
        manager: AccountId,
        previous_version_account_id: AccountId,
    ) -> Self {
        Self {
            token_account_id,
            fee_account_id,
            manager,
            products: UnorderedMap::new(StorageKey::Products),
            products_cache: HashMap::default().into(),
            accounts: LookupMap::new(StorageKey::Accounts),
            fee_amount: 0,
            previous_version_account_id,
            #[cfg(feature = "integration-test")]
            time_scale: 1.0,
        }
    }
}
