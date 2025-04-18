use std::{collections::HashMap, convert::Into};

use near_sdk::{
    env,
    json_types::{I64, U128},
    near_bindgen, AccountId,
};
use sweat_jar_model::{
    api::AccountApi,
    data::{
        account::Account,
        jar::{AggregatedInterestView, AggregatedTokenAmountView, JarView},
        product::{Product, ProductId, Terms},
        score::Score,
    },
    interest::InterestCalculator,
    TokenAmount, UTC,
};

use super::model::{AccountScoreUpdate, ScoreConverter};
use crate::{
    common::event::{emit, EventKind, ScoreData},
    feature::account::view::DetailedJar,
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
impl AccountApi for Contract {
    fn get_jars_for_account(&self, account_id: AccountId) -> Vec<JarView> {
        if let Some(account) = self.try_get_account(&account_id) {
            return account
                .jars
                .iter()
                .flat_map(|(product_id, jar)| {
                    let detailed_jar = &DetailedJar(product_id.clone(), jar.clone());
                    let views: Vec<JarView> = detailed_jar.into();
                    views
                })
                .collect();
        }

        vec![]
    }

    fn get_total_interest(&self, account_id: AccountId) -> AggregatedInterestView {
        if let Some(account) = self.try_get_account(&account_id) {
            return self.get_total_interest_for_account(account);
        }

        AggregatedInterestView::default()
    }

    fn unlock_jars_for_account(&mut self, account_id: AccountId) {
        self.assert_manager();

        let account = self.get_account_mut(&account_id);
        for jar in account.jars.values_mut() {
            jar.is_pending_withdraw = false;
        }
    }

    fn record_score(&mut self, batch: Vec<(AccountId, Vec<(Score, UTC)>)>) {
        self.assert_manager();

        let mut event = vec![];

        for (account_id, new_score) in batch {
            self.update_account_cache(
                &account_id,
                Some(|product: &Product| matches!(product.terms, Terms::ScoreBased(_))),
            );

            let account = self.get_account_mut(&account_id);
            account.score.try_reset_score();
            account.score.update(new_score.adjust(account.score.timezone));

            event.push(ScoreData {
                account_id,
                score: new_score,
            });
        }

        emit(EventKind::RecordScore(event));
    }

    fn get_timezone(&self, account_id: AccountId) -> Option<I64> {
        self.accounts
            .get(&account_id)
            .map(|account| I64(*account.score.timezone))
    }

    fn get_score(&self, account_id: AccountId) -> Option<U128> {
        let account = self.get_account(&account_id);

        Some(u128::from(account.score.active_score()).into())
    }
}
