#![cfg(test)]

use std::{
    borrow::Borrow,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use near_contract_standards::fungible_token::Balance;
use near_sdk::{test_utils::VMContextBuilder, testing_env, AccountId, NearToken};
use sweat_jar_model::{
    api::InitApi,
    data::product::{Product, ProductId},
    TokenAmount, MS_IN_DAY, MS_IN_HOUR, MS_IN_MINUTE,
};

use crate::{
    common::Timestamp,
    jar::{
        account::{versioned::AccountVersioned, Account},
        model::Jar,
    },
    test_utils::AfterCatchUnwind,
    Contract,
};

pub(crate) struct Context {
    contract: Arc<Mutex<Contract>>,
    pub owner: AccountId,
    ft_contract_id: AccountId,
    pub legacy_jar_contract_id: AccountId,
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
            manager,
            legacy_jar_contract_id.clone(),
        );

        Self {
            owner,
            ft_contract_id,
            builder,
            legacy_jar_contract_id,
            contract: Arc::new(Mutex::new(contract)),
        }
    }

    pub(crate) fn now(&self) -> Timestamp {
        self.builder.context.block_timestamp / 1_000_000
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

    pub(crate) fn with_jars(self, account_id: &AccountId, jars: &[(ProductId, Jar)]) -> Self {
        if jars.is_empty() {
            return self;
        }

        let mut account = Account::default();
        for (product_id, jar) in jars.iter() {
            account.jars.insert(product_id.clone(), jar.clone());
        }
        self.contract()
            .accounts
            .insert(account_id.clone(), AccountVersioned::new(account));

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

    pub(crate) fn with_deposit_yocto(&mut self, amount: Balance, f: impl FnOnce(&mut Context)) {
        self.set_deposit_yocto(amount);

        f(self);

        self.set_deposit_yocto(0);
    }

    pub(crate) fn set_deposit_yocto(&mut self, amount: Balance) {
        self.builder.attached_deposit(NearToken::from_yoctonear(amount));
        testing_env!(self.builder.build());
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
