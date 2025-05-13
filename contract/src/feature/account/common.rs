use std::ops::{Deref, DerefMut};

use near_sdk::{
    env::{self, panic_str},
    json_types::Base64VecU8,
    AccountId,
};
use sweat_jar_model::{
    data::{
        account::Account,
        deposit::{DepositTicket, Purpose},
        product::{Product, ProductAssertions, ProductId, Terms},
    },
    TokenAmount,
};

use crate::{
    common::event::{emit, EventKind},
    Contract,
};

impl Contract {
    pub(crate) fn deposit(
        &mut self,
        account_id: AccountId,
        ticket: DepositTicket,
        amount: TokenAmount,
        signature: &Option<Base64VecU8>,
    ) {
        let product_id = &ticket.product_id;
        let product = self.get_product(product_id);

        product.assert_enabled();
        product.assert_cap(amount);
        self.verify(Purpose::Deposit, &account_id, amount, &ticket, signature);

        let account = self.get_or_create_account_mut(&account_id);
        account.nonce += 1;

        if matches!(product.terms, Terms::ScoreBased(_)) {
            account.try_set_timezone(ticket.timezone);
        }

        account.deposit(product_id, amount, None);

        emit(EventKind::Deposit(account_id, (product_id.clone(), amount.into())));
    }

    pub(crate) fn try_get_account(&self, account_id: &AccountId) -> Option<&Account> {
        self.accounts.get(account_id).map(Deref::deref)
    }

    pub(crate) fn get_account(&self, account_id: &AccountId) -> &Account {
        self.try_get_account(account_id)
            .unwrap_or_else(|| panic_str(format!("Account {account_id} is not found").as_str()))
    }

    pub(crate) fn get_account_mut(&mut self, account_id: &AccountId) -> &mut Account {
        self.accounts
            .get_mut(account_id)
            .unwrap_or_else(|| panic_str(format!("Account {account_id} is not found").as_str()))
            .deref_mut()
    }

    pub(crate) fn get_or_create_account_mut(&mut self, account_id: &AccountId) -> &mut Account {
        self.accounts.entry(account_id.clone()).or_default()
    }

    pub(crate) fn update_account_cache(&mut self, account_id: &AccountId, filter: Option<fn(&Product) -> bool>) {
        let now = env::block_timestamp_ms();
        let products = self.get_products_for_account(account_id, filter);
        let account = self.get_account_mut(account_id);

        for product in products {
            account.update_jar_cache(&product, now);
        }
    }

    pub(crate) fn update_jar_cache(&mut self, account_id: &AccountId, product_id: &ProductId) {
        let product = &self.get_product(product_id);
        let account = self.get_account_mut(account_id);
        account.update_jar_cache(product, env::block_timestamp_ms());
    }
}
