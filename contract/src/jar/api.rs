use std::{collections::HashMap, convert::Into};

use near_sdk::{env, json_types::U128, near_bindgen, AccountId};
use sweat_jar_model::{
    api::JarApi,
    data::jar::{AggregatedInterestView, AggregatedTokenAmountView, JarView},
    data::product::ProductId,
    TokenAmount,
};

use crate::{
    assert::assert_not_locked_legacy,
    jar::{account::Account, model::AccountLegacyV3, view::DetailedJarV2},
    product::model::v1::InterestCalculator,
    Contract, ContractExt,
};

impl Contract {
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
            let account = self.map_legacy_account(&account);
            return self.get_total_interest_for_account(&account);
        }

        AggregatedInterestView::default()
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

pub(crate) type MigratingAccount = (Account, HashMap<ProductId, TokenAmount>);

impl From<&AccountLegacyV3> for MigratingAccount {
    fn from(value: &AccountLegacyV3) -> Self {
        let mut account = Account {
            nonce: value.last_id,
            score: value.score,
            ..Account::default()
        };

        let mut claimed_balances = HashMap::<ProductId, TokenAmount>::new();

        for jar in &value.jars {
            assert_not_locked_legacy(jar);

            account.deposit(&jar.product_id, jar.principal, jar.created_at.into());

            *claimed_balances.entry(jar.product_id.clone()).or_insert(0) += jar.claimed_balance;

            if !account.is_penalty_applied {
                account.is_penalty_applied = jar.is_penalty_applied;
            }
        }

        (account, claimed_balances)
    }
}
