#![cfg(test)]

use std::{
    borrow::Borrow,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use near_contract_standards::fungible_token::Balance;
use near_sdk::{env::block_timestamp_ms, test_utils::VMContextBuilder, testing_env, AccountId, NearToken};
use sweat_jar_model::{api::InitApi, MS_IN_DAY, MS_IN_HOUR, MS_IN_MINUTE};

use crate::{jar::model::Jar, product::model::Product, test_utils::AfterCatchUnwind, Contract};

pub(crate) struct Context {
    contract: Arc<Mutex<Contract>>,
    pub owner: AccountId,
    ft_contract_id: AccountId,
    builder: VMContextBuilder,
}

impl Context {
    pub(crate) fn new(manager: AccountId) -> Self {
        let owner: AccountId = "owner".to_string().try_into().unwrap();
        let fee_account_id: AccountId = "fee".to_string().try_into().unwrap();
        let ft_contract_id: AccountId = "token".to_string().try_into().unwrap();

        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(owner.clone())
            .signer_account_id(owner.clone())
            .predecessor_account_id(owner.clone())
            .block_timestamp(0);

        testing_env!(builder.build());

        let contract = Contract::init(ft_contract_id.clone(), fee_account_id, manager);

        Self {
            owner,
            ft_contract_id,
            builder,
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

    pub(crate) fn with_jars(self, jars: &[Jar]) -> Self {
        if jars.is_empty() {
            return self;
        }

        let max_id = jars.iter().map(|j| j.id).max().unwrap();

        for jar in jars {
            self.contract()
                .account_jars
                .entry(jar.account_id.clone())
                .or_default()
                .push(jar.clone());
        }

        if max_id > self.contract().last_jar_id {
            self.contract().last_jar_id = max_id;
        }

        self
    }

    pub(crate) fn set_block_timestamp_today(&mut self) {
        let start = SystemTime::now();
        let today = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
        self.set_block_timestamp(today);
    }

    pub(crate) fn advance_block_timestamp_days(&mut self, days: u64) {
        let now = block_timestamp_ms();
        self.set_block_timestamp_in_ms(now + days * MS_IN_DAY);
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
        self.switch_account(&self.ft_contract_id.clone());
    }

    pub(crate) fn with_deposit_yocto(&mut self, amount: Balance, f: impl FnOnce(&mut Context)) {
        self.set_deposit_yocto(amount);

        f(self);

        self.set_deposit_yocto(0);
    }

    fn set_deposit_yocto(&mut self, amount: Balance) {
        self.builder.attached_deposit(NearToken::from_yoctonear(amount));
        testing_env!(self.builder.build());
    }
}

impl AfterCatchUnwind for Context {
    fn after_catch_unwind(&self) {
        self.contract.clear_poison();
    }
}
