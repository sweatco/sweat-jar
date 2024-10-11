use std::collections::HashMap;

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
    product::model::v2::ProductV2,
    Contract, ContractExt,
};

impl Contract {
    fn restake_internal(&mut self, product: &ProductV2) {
        require!(product.is_enabled, "The product is disabled");

        let account_id = env::predecessor_account_id();
        let account = self.get_account_mut(&account_id);
        let jar = account.get_jar_mut(&product.id);

        let now = env::block_timestamp_ms();
        let (amount, partition_index) = jar.get_liquid_balance(&product.terms, now);

        require!(amount > 0, "Nothing to restake");

        self.update_jar_cache(account, &product.id);
        jar.clean_up_deposits(partition_index);
        account.deposit(&product.id, amount);
    }
}

#[near_bindgen]
impl JarApi for Contract {
    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView> {
        if let Some(account) = self.try_get_account(&account_id) {
            return account
                .jars
                .iter()
                .flat_map(|(product_id, jar)| DetailedJarV2(product_id.clone(), jar.clone()).into())
                .collect();
        }

        if let Some(jars) = self.account_jars(&account_id) {
            return jars.iter().map(Into::into).collect();
        }

        vec![]
    }

    // TODO: add v2 support
    fn get_total_interest(&self, account_id: AccountId) -> AggregatedInterestView {
        if let Some(account) = self.try_get_account(&account_id) {
            return account.get_total_interest();
        }

        if let Some(account) = self.get_account_legacy(&account_id) {
            return AccountV2::from(account).get_total_interest();
        }

        AggregatedInterestView::default()
    }

    fn restake(&mut self, product_id: ProductId) {
        self.migrate_account_if_needed(&env::predecessor_account_id());

        self.restake_internal(&self.get_product(&product_id));

        // TODO: add event logging
    }

    fn restake_all(&mut self, product_ids: Option<Vec<ProductId>>) {
        let account_id = env::predecessor_account_id();

        self.migrate_account_if_needed(&account_id);

        let product_ids = product_ids.unwrap_or_else(|| {
            self.get_account(&account_id)
                .jars
                .keys()
                .filter(|product_id| self.get_product(product_id).is_enabled)
                .collect()
        });
        for product_id in product_ids.iter() {
            self.restake_internal(&self.get_product(product_id));
        }

        // TODO: add event logging
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

impl AccountV2 {
    fn get_total_interest(&self) -> AggregatedInterestView {
        let mut detailed_amounts = HashMap::<JarIdView, U128>::new();
        let mut total_amount: TokenAmount = 0;

        for (product_id, jar) in self.jars {
            let product = self.get_product(&product_id);
            let interest = product.terms.get_interest(self, &jar).0;

            detailed_amounts.insert(product_id, interest.into());
            total_amount += interest;
        }

        AggregatedInterestView {
            amount: AggregatedTokenAmountView {
                detailed: detailed_amounts,
                total: U128(total_amount),
            },
            timestamp: env::block_timestamp_ms(),
        };
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

        for jar in value.jars {
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
