#![cfg(test)]

use std::time::Duration;

use model::api::InitApi;
use near_sdk::{test_utils::VMContextBuilder, testing_env, AccountId, Balance};

use crate::{jar::model::Jar, product::model::Product, Contract};

pub(crate) struct Context {
    pub contract: Contract,
    pub owner: AccountId,
    ft_contract_id: AccountId,
    builder: VMContextBuilder,
}

impl Context {
    pub(crate) fn new(manager: AccountId) -> Self {
        let owner = AccountId::new_unchecked("owner".to_string());
        let fee_account_id = AccountId::new_unchecked("fee".to_string());
        let ft_contract_id = AccountId::new_unchecked("token".to_string());

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
            contract,
        }
    }

    pub(crate) fn with_products(mut self, products: &[Product]) -> Self {
        for product in products {
            self.contract.products.insert(product.id.clone(), product.clone());
        }

        self
    }

    pub(crate) fn with_jars(mut self, jars: &[Jar]) -> Self {
        for jar in jars {
            self.contract
                .account_jars
                .entry(jar.account_id.clone())
                .or_default()
                .push(jar.clone());
        }

        self
    }

    pub(crate) fn set_block_timestamp_in_days(&mut self, days: u64) {
        self.set_block_timestamp(Duration::from_secs(days * 24 * 60 * 60));
    }

    pub(crate) fn set_block_timestamp_in_minutes(&mut self, hours: u64) {
        self.set_block_timestamp(Duration::from_secs(hours * 60));
    }

    pub(crate) fn set_block_timestamp_in_ms(&mut self, ms: u64) {
        self.set_block_timestamp(Duration::from_millis(ms));
    }

    pub(crate) fn set_block_timestamp(&mut self, duration: Duration) {
        self.builder.block_timestamp(duration.as_nanos() as u64);
        testing_env!(self.builder.build());
    }

    pub(crate) fn switch_account(&mut self, account_id: &AccountId) {
        self.builder
            .predecessor_account_id(account_id.clone())
            .signer_account_id(account_id.clone());
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
        self.builder.attached_deposit(amount);
        testing_env!(self.builder.build());
    }
}
