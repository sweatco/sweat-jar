use std::{collections::HashMap, convert::Into};

use near_sdk::{env, json_types::U128, near_bindgen, require, AccountId};
use sweat_jar_model::{
    api::JarApi,
    jar::{AggregatedInterestView, AggregatedTokenAmountView, JarView},
    ProductId, TokenAmount,
};

use crate::{
    assert::assert_not_locked_legacy,
    event::{emit, EventKind, RestakeData},
    jar::{account::Account, model::AccountLegacyV2, view::DetailedJarV2},
    product::model::v1::{InterestCalculator, Product},
    score::AccountScore,
    Contract, ContractExt,
};

impl Contract {
    fn restake_internal(&mut self, account_id: &AccountId, product: &Product) -> Option<TokenAmount> {
        require!(product.is_enabled, "The product is disabled");

        let jar = self.get_account(account_id).get_jar(&product.id);

        let (amount, partition_index) = jar.get_liquid_balance(&product.terms);

        if amount == 0 {
            return None;
        }

        self.update_jar_cache(account_id, &product.id);

        let account = self.get_account_mut(account_id);
        let jar = account.get_jar_mut(&product.id);
        jar.clean_up_deposits(partition_index);
        account.deposit(&product.id, amount, None);

        Some(amount)
    }

    fn get_total_interest_for_account(&self, account: &Account) -> AggregatedInterestView {
        let mut detailed_amounts = HashMap::<ProductId, U128>::new();
        let mut total_amount: TokenAmount = 0;

        for (product_id, jar) in &account.jars {
            let product = self.get_product(product_id);
            let interest = product.terms.get_interest(account, jar, env::block_timestamp_ms()).0;

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

        if let Some(jars) = self.archive.get_jars(&account_id) {
            return jars.iter().map(Into::into).collect();
        }

        vec![]
    }

    fn get_total_interest(&self, account_id: AccountId) -> AggregatedInterestView {
        if let Some(account) = self.try_get_account(&account_id) {
            return self.get_total_interest_for_account(account);
        }

        if let Some(account) = self.archive.get_account(&account_id) {
            return self.get_total_interest_for_account(&Account::from(&account));
        }

        AggregatedInterestView::default()
    }

    fn restake(&mut self, product_id: ProductId) {
        self.assert_migrated(&env::predecessor_account_id());

        let result = self.restake_internal(&env::predecessor_account_id(), &self.get_product(&product_id));

        if let Some(amount) = result {
            emit(EventKind::Restake(RestakeData::new(product_id, amount)));
        } else {
            require!(result.is_some(), "Nothing to restake");
        }
    }

    fn unlock_jars_for_account(&mut self, account_id: AccountId) {
        self.assert_manager();
        self.assert_migrated(&account_id);

        let account = self.get_account_mut(&account_id);
        for jar in account.jars.values_mut() {
            jar.is_pending_withdraw = false;
        }
    }
}

impl From<&AccountLegacyV2> for Account {
    fn from(value: &AccountLegacyV2) -> Self {
        let mut account = Account {
            nonce: value.last_id,
            jars: HashMap::default(),
            score: AccountScore::default(),
            is_penalty_applied: false,
        };

        for jar in &value.jars {
            assert_not_locked_legacy(jar);

            account.deposit(&jar.product_id, jar.principal, jar.created_at.into());
            account.get_jar_mut(&jar.product_id).claimed_balance += jar.claimed_balance;

            if !account.is_penalty_applied {
                account.is_penalty_applied = jar.is_penalty_applied;
            }
        }

        account
    }
}
