use std::collections::HashMap;

use near_sdk::{env, env::panic_str, json_types::U128, near_bindgen, require, AccountId};
use sweat_jar_model::{
    api::JarApi,
    jar::{AggregatedInterestView, AggregatedTokenAmountView, JarId, JarIdView, JarView},
    ProductId, TokenAmount, JAR_BATCH_SIZE, U32,
};

use crate::{
    event::{emit, EventKind},
    jar::model::Jar,
    score::AccountScore,
    Contract, ContractExt, JarsStorage,
};

impl Contract {
    fn restake_internal(&mut self, product_id: &ProductId) {
        let product = self.get_product(product_id);
        require!(product.is_enabled, "The product is disabled");

        let account_id = env::predecessor_account_id();
        let account = self.get_account_mut(&account_id);
        let jar = account.get_jar_mut(product_id);

        let now = env::block_timestamp_ms();
        let (amount, partition_index) = jar.get_liquid_balance(product.terms, now);

        require!(amount > 0, "Nothing to restake");

        // TODO: use update for a single jar
        self.update_cache(account);

        // TODO: extract method and use in `clean_up()`
        if partition_index == jar.deposits.len() {
            jar.deposits.clear();
        } else {
            jar.deposits.drain(..partition_index);
        }

        account.deposit(product_id, amount);
    }
}

#[near_bindgen]
impl JarApi for Contract {
    // TODO: restore previous version after V2 migration
    // TODO: add v2 support
    #[mutants::skip]
    fn get_jar(&self, account_id: AccountId, jar_id: JarIdView) -> JarView {
        if let Some(record) = self.account_jars_v1.get(&account_id) {
            let jar: Jar = record
                .jars
                .iter()
                .find(|jar| jar.id == jar_id.0)
                .unwrap_or_else(|| env::panic_str(&format!("Jar with id: {} doesn't exist", jar_id.0)))
                .clone()
                .into();

            return jar.into();
        }

        if let Some(record) = self.account_jars_non_versioned.get(&account_id) {
            let jar: Jar = record
                .jars
                .iter()
                .find(|jar| jar.id == jar_id.0)
                .unwrap_or_else(|| env::panic_str(&format!("Jar with id: {} doesn't exist", jar_id.0)))
                .clone();

            return jar.into();
        }

        self.accounts
            .get(&account_id)
            .unwrap_or_else(|| panic_str(&format!("Account '{account_id}' doesn't exist")))
            .get_jar(jar_id.0)
            .into()
    }

    // TODO: add v2 support
    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView> {
        self.account_jars(&account_id).iter().map(Into::into).collect()
    }

    // TODO: add v2 support
    fn get_total_principal(&self, account_id: AccountId) -> AggregatedTokenAmountView {
        self.get_principal(
            self.account_jars(&account_id).iter().map(|a| U32(a.id)).collect(),
            account_id,
        )
    }

    // TODO: add v2 support
    fn get_principal(&self, jar_ids: Vec<JarIdView>, account_id: AccountId) -> AggregatedTokenAmountView {
        let mut detailed_amounts = HashMap::<JarIdView, U128>::new();
        let mut total_amount: TokenAmount = 0;

        for jar in self.account_jars_with_ids(&account_id, &jar_ids) {
            let id = jar.id;
            let principal = jar.principal;

            detailed_amounts.insert(U32(id), U128(principal));
            total_amount += principal;
        }

        AggregatedTokenAmountView {
            detailed: detailed_amounts,
            total: U128(total_amount),
        }
    }

    // TODO: add v2 support
    fn get_total_interest(&self, account_id: AccountId) -> AggregatedInterestView {
        self.get_interest(
            self.account_jars(&account_id).iter().map(|a| U32(a.id)).collect(),
            account_id,
        )
    }

    // TODO: add v2 support
    fn get_interest(&self, jar_ids: Vec<JarIdView>, account_id: AccountId) -> AggregatedInterestView {
        let now = env::block_timestamp_ms();

        let mut detailed_amounts = HashMap::<JarIdView, U128>::new();
        let mut total_amount: TokenAmount = 0;

        let score = self
            .get_score(&account_id)
            .map(AccountScore::claimable_score)
            .unwrap_or_default();

        for jar in self.account_jars_with_ids(&account_id, &jar_ids) {
            let product = self.get_product(&jar.product_id);

            let interest = jar.get_interest(&score, &product, now).0;

            detailed_amounts.insert(U32(jar.id), U128(interest));
            total_amount += interest;
        }

        AggregatedInterestView {
            amount: AggregatedTokenAmountView {
                detailed: detailed_amounts,
                total: U128(total_amount),
            },
            timestamp: now,
        }
    }

    fn restake(&mut self, product_id: ProductId) {
        self.migrate_account_if_needed(&env::predecessor_account_id());
        self.restake_internal(&product_id);

        // TODO: add event logging
    }

    fn restake_all(&mut self, product_ids: Option<Vec<ProductId>>) {
        let account_id = env::predecessor_account_id();

        self.migrate_account_if_needed(&account_id);

        let product_ids = product_ids.unwrap_or_else(|| self.get_account(&account_id).jars.keys().collect());
        for product_id in product_ids {
            self.restake_internal(&product_id);
        }

        // TODO: add event logging
    }

    // TODO: add v2 support
    fn unlock_jars_for_account(&mut self, account_id: AccountId) {
        self.assert_manager();
        self.migrate_account_if_needed(&account_id);

        let jars = self.accounts.get_mut(&account_id).expect("Account doesn't have jars");

        for jar in &mut jars.jars {
            jar.is_pending_withdraw = false;
        }
    }
}
