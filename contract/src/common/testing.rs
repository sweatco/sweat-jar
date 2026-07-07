#![cfg(test)]

use std::{
    borrow::Borrow,
    panic::{catch_unwind, UnwindSafe},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use near_contract_standards::fungible_token::Balance;
use near_plugins::AccessControllable;
use near_sdk::{
    borsh::to_vec, json_types::Base64VecU8, test_utils::VMContextBuilder, testing_env, AccountId, NearToken,
    PromiseOrValue,
};
use sweat_jar_model::{
    api::{InitApi, RoleAssignments},
    data::{
        account::{v1::AccountV1, versioned::AccountVersioned, Account},
        jar::Jar,
        product::{Product, ProductId},
    },
    TokenAmount, MS_IN_DAY, MS_IN_HOUR, MS_IN_MINUTE,
};

use super::{env::test_env_ext, event::EventKind};
use crate::{migration::api::store_account_raw, Contract};

pub mod accounts {
    use near_sdk::AccountId;
    use rstest::fixture;

    #[fixture]
    pub fn admin() -> AccountId {
        "admin.near".parse().unwrap()
    }

    #[fixture]
    pub fn alice() -> AccountId {
        near_sdk::test_utils::test_env::alice()
    }

    #[fixture]
    pub fn bob() -> AccountId {
        near_sdk::test_utils::test_env::bob()
    }

    #[fixture]
    pub fn carol() -> AccountId {
        near_sdk::test_utils::test_env::carol()
    }
}

pub(crate) struct Context {
    contract: Arc<Mutex<Contract>>,
    pub owner: AccountId,
    pub ft_contract_id: AccountId,
    pub legacy_jar_contract_id: AccountId,
    pub manager: AccountId,
    builder: VMContextBuilder,
}

impl Context {
    pub(crate) fn new(manager: AccountId) -> Self {
        let owner: AccountId = "owner".to_string().try_into().unwrap();
        let fee_account_id: AccountId = "fee".to_string().try_into().unwrap();
        let ft_contract_id: AccountId = "token".to_string().try_into().unwrap();
        let legacy_jar_contract_id: AccountId = "legacy_jar".to_string().try_into().unwrap();

        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(owner.clone())
            .signer_account_id(owner.clone())
            .predecessor_account_id(owner.clone())
            .block_timestamp(0);

        testing_env!(builder.build());

        let contract = Contract::init(
            ft_contract_id.clone(),
            fee_account_id,
            legacy_jar_contract_id.clone(),
            manager.clone(),
            RoleAssignments {
                oracle: vec![manager.clone()],
                product_manager: vec![manager.clone()],
                fee_manager: vec![manager.clone()],
                maintainer: vec![manager.clone()],
                staging_manager: vec![manager.clone()],
                upgrade_manager: vec![manager.clone()],
            },
        );

        Self {
            owner,
            ft_contract_id,
            builder,
            legacy_jar_contract_id,
            manager,
            contract: Arc::new(Mutex::new(contract)),
        }
    }

    pub(crate) fn contract(&self) -> MutexGuard<Contract> {
        self.contract.try_lock().expect("Contract is already locked")
    }

    pub(crate) fn with_products(self, products: &[Product]) -> Self {
        for product in products {
            self.contract().products.insert(&product.id, product);
        }

        self
    }

    pub(crate) fn with_latest_account(self, account_id: &AccountId, jars: &[(ProductId, Jar)]) -> Self {
        if jars.is_empty() {
            return self;
        }

        let mut account = Account::default();
        for (product_id, jar) in jars {
            account.jars.insert(product_id.clone(), jar.clone());
        }

        store_account_raw(
            account_id.clone(),
            Base64VecU8(to_vec(&AccountVersioned::new(account)).unwrap()),
        );

        self
    }

    pub(crate) fn with_v1_account(self, account_id: &AccountId, jars: &[(ProductId, Jar)]) -> Self {
        if jars.is_empty() {
            return self;
        }

        let mut account = AccountV1::default();
        for (product_id, jar) in jars {
            account.jars.insert(product_id.clone(), jar.clone());
        }

        store_account_raw(
            account_id.clone(),
            Base64VecU8(to_vec(&AccountVersioned::V1(account)).unwrap()),
        );

        self
    }

    pub(crate) fn set_block_timestamp_in_days(&mut self, days: u64) {
        self.set_block_timestamp(Duration::from_millis(days * MS_IN_DAY));
    }

    pub(crate) fn set_block_timestamp_in_minutes(&mut self, minutes: u64) {
        self.set_block_timestamp(Duration::from_millis(minutes * MS_IN_MINUTE));
    }

    pub(crate) fn set_block_timestamp_in_hours(&mut self, hours: u64) {
        self.set_block_timestamp(Duration::from_millis(hours * MS_IN_HOUR));
    }

    pub(crate) fn set_block_timestamp_in_ms(&mut self, ms: u64) {
        self.set_block_timestamp(Duration::from_millis(ms));
    }

    fn set_block_timestamp(&mut self, duration: Duration) {
        self.builder.block_timestamp(duration.as_nanos() as u64);
        testing_env!(self.builder.build());
    }

    pub(crate) fn switch_account(&mut self, account_id: impl Borrow<AccountId>) {
        let account_id = account_id.borrow().clone();
        self.builder
            .predecessor_account_id(account_id.clone())
            .signer_account_id(account_id);
        testing_env!(self.builder.build());
    }

    pub(crate) fn switch_account_to_ft_contract_account(&mut self) {
        self.switch_account(self.ft_contract_id.clone());
    }

    pub(crate) fn switch_account_to_manager(&mut self) {
        let manager = self.manager.clone();
        self.switch_account(manager);
    }

    pub(crate) fn with_deposit_yocto(&mut self, amount: Balance, f: impl FnOnce(&mut Context)) {
        self.set_deposit_yocto(amount);

        f(self);

        self.set_deposit_yocto(0);
    }

    pub(crate) fn set_deposit_yocto(&mut self, amount: Balance) {
        self.builder.attached_deposit(NearToken::from_yoctonear(amount));
        testing_env!(self.builder.build());
    }

    pub(crate) fn get_events(&self) -> Vec<EventKind> {
        test_env_ext::get_events()
    }
}

impl AfterCatchUnwind for Context {
    fn after_catch_unwind(&self) {
        self.contract.clear_poison();
    }
}

pub trait TokenUtils {
    fn to_otto(&self) -> TokenAmount;
}

impl TokenUtils for u128 {
    fn to_otto(&self) -> TokenAmount {
        self * 10u128.pow(18)
    }
}

pub trait WhitespaceTrimmer {
    fn trim_whitespaces(&self) -> String;
}

impl WhitespaceTrimmer for &str {
    fn trim_whitespaces(&self) -> String {
        let words: Vec<_> = self.split_whitespace().collect();
        words.join(" ")
    }
}

impl WhitespaceTrimmer for String {
    fn trim_whitespaces(&self) -> String {
        self.as_str().trim_whitespaces()
    }
}

pub trait AfterCatchUnwind {
    fn after_catch_unwind(&self);
}

impl AfterCatchUnwind for () {
    fn after_catch_unwind(&self) {}
}

pub fn expect_panic(ctx: &impl AfterCatchUnwind, msg: &str, action: impl FnOnce() + UnwindSafe) {
    let res = catch_unwind(action);

    let panic_msg = res
        .err()
        .unwrap_or_else(|| panic!("Contract didn't panic when expected to.\nExpected message: {msg}"));

    if msg.is_empty() {
        ctx.after_catch_unwind();
        return;
    }

    let panic_msg = if let Some(msg) = panic_msg.downcast_ref::<&str>() {
        (*msg).to_string()
    } else if let Some(msg) = panic_msg.downcast_ref::<String>() {
        msg.clone()
    } else {
        panic!("Contract didn't panic with String or &str.\nExpected message: {msg}")
    };

    assert!(
        panic_msg.contains(msg),
        "Expected panic message to contain: {msg}.\nPanic message: {panic_msg}"
    );

    ctx.after_catch_unwind();
}

pub trait UnwrapPromise<T> {
    fn unwrap(self) -> T;
}

impl<T> UnwrapPromise<T> for PromiseOrValue<T> {
    fn unwrap(self) -> T {
        let PromiseOrValue::Value(t) = self else {
            panic!("Failed to unwrap PromiseOrValue")
        };
        t
    }
}

#[cfg(test)]
mod tests {
    use crate::common::testing::{expect_panic, AfterCatchUnwind};

    #[test]
    #[should_panic(expected = "Contract didn't panic when expected to.\nExpected message: Something went wrong")]
    fn test_expect_panic() {
        struct Ctx;
        impl AfterCatchUnwind for Ctx {
            fn after_catch_unwind(&self) {}
        }

        expect_panic(&Ctx, "Something went wrong", || {
            panic!("{}", "Something went wrong");
        });

        expect_panic(&Ctx, "Something went wrong", || {});
    }
}

#[cfg(feature = "integration-test")]
mod integration_tests {
    use std::{cell::RefCell, collections::HashMap};

    use near_sdk::{
        borsh::{BorshDeserialize, BorshSerialize},
        collections::UnorderedMap,
        near,
        store::LookupMap,
        AccountId,
    };
    use sweat_jar_model::{
        data::{
            account::versioned::AccountVersioned,
            product::{Product, ProductId},
        },
        TokenAmount,
    };

    use crate::{feature::booster::model::Boosters, Contract};

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
                boosters: Boosters::new(
                    env::block_timestamp_ms(),
                    StorageKey::BoostersIndex,
                    StorageKey::BoostersItems,
                ),
                time_scale: 1.0,
            }
        }
    }
    #[near(serializers=[borsh])]
    struct ContractSerdeHelper {
        token_account_id: AccountId,
        fee_account_id: AccountId,
        manager: AccountId,
        products: UnorderedMap<ProductId, Product>,
        accounts: LookupMap<AccountId, AccountVersioned>,
        fee_amount: TokenAmount,
        previous_version_account_id: AccountId,
        boosters: Boosters,
        time_scale: f64,
    }

    impl From<ContractSerdeHelper> for Contract {
        fn from(value: ContractSerdeHelper) -> Self {
            Self {
                token_account_id: value.token_account_id,
                fee_account_id: value.fee_account_id,
                manager: value.manager,
                products: value.products,
                accounts: value.accounts,
                products_cache: RefCell::new(HashMap::new()),
                fee_amount: value.fee_amount,
                previous_version_account_id: value.previous_version_account_id,
                boosters: value.boosters,
                time_scale: value.time_scale,
            }
        }
    }

    impl BorshSerialize for Contract {
        fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
            // Directly serialize all fields in the same order as ContractSerdeHelper
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

    impl BorshDeserialize for Contract {
        fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let state = ContractSerdeHelper::deserialize_reader(reader)?;

            // Sync time scale to global thread-local storage automatically on deserialization
            sweat_jar_model::set_global_time_scale(state.time_scale);

            Ok(state.into())
        }
    }
}
