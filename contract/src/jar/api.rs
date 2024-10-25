use std::{collections::HashMap, convert::Into, ops::Deref};

use near_sdk::{env, json_types::U128, near_bindgen, require, AccountId};
use sweat_jar_model::{
    api::JarApi,
    jar::{AggregatedInterestView, AggregatedTokenAmountView, JarIdView, JarView},
    ProductId, TokenAmount,
};

use crate::{
    jar::{
        account::{v1::AccountV1, v2::AccountV2},
        model::Deposit,
        view::DetailedJarV2,
    },
    product::model::v2::{InterestCalculator, ProductV2},
    Contract, ContractExt,
};

impl Contract {
    fn restake_internal(&mut self, product: &ProductV2) -> Option<TokenAmount> {
        require!(product.is_enabled, "The product is disabled");

        let account_id = env::predecessor_account_id();
        let now = env::block_timestamp_ms();

        let account = self.get_account(&account_id);
        let jar = account.get_jar(&product.id);

        let (amount, partition_index) = jar.get_liquid_balance(&product.terms, now);

        if amount == 0 {
            return None;
        }

        self.update_jar_cache(&account_id, &product.id);

        let account = self.get_account_mut(&account_id);
        let jar = account.get_jar_mut(&product.id);
        jar.clean_up_deposits(partition_index);
        account.deposit(&product.id, amount);

        Some(amount)
    }

    fn get_total_interest_for_account(&self, account: &AccountV2) -> AggregatedInterestView {
        let mut detailed_amounts = HashMap::<ProductId, U128>::new();
        let mut total_amount: TokenAmount = 0;

        for (product_id, jar) in account.jars.iter() {
            let product = self.get_product(product_id);
            let interest = product.terms.get_interest(account, &jar, env::block_timestamp_ms()).0;

            detailed_amounts.insert(product_id.clone(), interest.into());
            total_amount += interest;
        }

        AggregatedInterestView {
            amount: AggregatedTokenAmountView {
                detailed: detailed_amounts,
                total: U128(total_amount),
            },
            timestamp: env::block_timestamp_ms(),
        }
    }
}

#[near_bindgen]
impl JarApi for Contract {
    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView> {
        if let Some(account) = self.try_get_account(&account_id) {
            return account
                .jars
                .iter()
                .flat_map(|(product_id, jar)| {
                    let detailed_jar = &DetailedJarV2(product_id.clone(), jar.clone());
                    let views: Vec<JarView> = detailed_jar.into();
                    views
                })
                .collect();
        }

        if let Some(jars) = self.get_legacy_account_jars(&account_id) {
            return jars.iter().map(Into::into).collect();
        }

        vec![]
    }

    fn get_total_interest(&self, account_id: AccountId) -> AggregatedInterestView {
        if let Some(account) = self.try_get_account(&account_id) {
            return self.get_total_interest_for_account(account);
        }

        if let Some(account) = self.get_account_legacy(&account_id) {
            return self.get_total_interest_for_account(&AccountV2::from(account.deref()));
        }

        AggregatedInterestView::default()
    }

    fn restake(&mut self, product_id: ProductId) {
        self.migrate_account_if_needed(&env::predecessor_account_id());

        let result = self.restake_internal(&self.get_product(&product_id));
        require!(result.is_some(), "Nothing to restake");

        // TODO: add event logging
    }

    fn restake_all(&mut self, product_ids: Option<Vec<ProductId>>) -> Vec<(ProductId, TokenAmount)> {
        let account_id = env::predecessor_account_id();

        self.migrate_account_if_needed(&account_id);

        let products: Vec<ProductV2> = product_ids
            .unwrap_or_else(|| {
                self.get_account(&account_id)
                    .jars
                    .keys()
                    .cloned()
                    .collect::<Vec<ProductId>>()
            })
            .iter()
            .map(|product_id| self.get_product(product_id))
            .filter(|product| product.is_enabled)
            .collect();

        let mut result: Vec<(ProductId, TokenAmount)> = vec![];
        for product in products.iter() {
            if let Some(amount) = self.restake_internal(product) {
                result.push((product.id.clone(), amount));
            }
        }

        // TODO: add event logging

        result
    }

    fn unlock_jars_for_account(&mut self, account_id: AccountId) {
        self.assert_manager();
        self.migrate_account_if_needed(&account_id);

        let account = self.get_account_mut(&account_id);
        for (_, jar) in account.jars.iter_mut() {
            jar.is_pending_withdraw = false;
        }
    }
}

impl From<&AccountV1> for AccountV2 {
    fn from(value: &AccountV1) -> Self {
        let mut account = AccountV2 {
            nonce: value.last_id,
            jars: Default::default(),
            score: value.score,
            is_penalty_applied: false,
        };

        for jar in value.jars.iter() {
            let deposit = Deposit::new(jar.created_at, jar.principal);
            account.push(&jar.product_id, deposit);

            if !account.is_penalty_applied {
                account.is_penalty_applied = jar.is_penalty_applied;
            }
        }

        // TODO: update and migrate cache
        // TODO: migrate remainders

        account
    }
}
