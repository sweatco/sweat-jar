use std::{collections::HashMap, convert::Into};

use near_sdk::{
    env::{self, panic_str},
    json_types::{I64, U128},
    near, AccountId,
};
use sweat_jar_model::{
    api::AccountApi,
    convert_to_days_offset,
    data::{
        account::{common::FeaturesAccess, features::Feature, view::AccountView, Account},
        jar::{AggregatedInterestView, AggregatedTokenAmountView, JarsView},
        product::{Product, ProductId, Terms},
        score::Score,
    },
    interest::InterestCalculator,
    DaysOffset, ScoreIncrementProcessor, TimeHelper, Timezone, TokenAmount, UTC,
};

use crate::{
    common::event::{emit, ApplyBoosterData, EventKind, ScoreData},
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

    fn update_score_based_jars_cache(&mut self, account_id: &AccountId) {
        self.update_account_cache(
            account_id,
            Some(|product: &Product| {
                matches!(product.terms, Terms::ScoreBased(_)) || matches!(product.terms, Terms::TieredScoreBased(_))
            }),
        );
    }
}

#[near]
impl AccountApi for Contract {
    fn get_jars_for_account(&self, account_id: AccountId) -> JarsView {
        if let Some(account) = self.try_get_account(&account_id) {
            return account.into();
        }

        JarsView::default()
    }

    fn get_account(&self, account_id: AccountId) -> Option<AccountView> {
        self.try_get_account(&account_id).map(|account| account.clone().into())
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

        for (account_id, increments) in batch {
            self.assert_timezone_is_set(&account_id);
            self.update_score_based_jars_cache(&account_id);

            let account = self.get_account_mut(&account_id);
            account.score.settle(account.timezone);
            account.assert_no_pending_score();

            let segmented_increments = ScoreIncrementProcessor::new(&increments, account.timezone).process();
            let normalized_increments = convert_to_days_offset(segmented_increments.valid.clone(), account.timezone);
            account.score.update(normalized_increments);

            for increment in segmented_increments.outdated {
                emit(EventKind::OldScoreWarning(increment));
            }

            event.push(ScoreData {
                account_id,
                score: segmented_increments.valid,
            });
        }

        emit(EventKind::RecordScore(event));
    }

    fn apply_booster(&mut self, account_ids: Vec<AccountId>, score: Score, timestamp: UTC) {
        self.assert_manager();

        let mut applied = vec![];
        let mut rejected = vec![];

        for account_id in &account_ids {
            self.assert_timezone_is_set(account_id);
            self.get_account(account_id).timezone.assert_not_future(timestamp);

            self.update_score_based_jars_cache(account_id);

            let account = self.get_account_mut(account_id);
            account.score.settle(account.timezone);
            account.assert_no_pending_score();

            let adjusted_timestamp = account.timezone.adjust(timestamp);
            let days_offset = DaysOffset::try_from(account.timezone.today().0 - adjusted_timestamp.day().0)
                .unwrap_or_else(|_| panic_str("Failed to calculate days offset"));

            if account.score.apply_booster(days_offset, score) {
                applied.push(account_id.clone());
            } else {
                rejected.push(account_id.clone());
            }
        }

        emit(EventKind::ApplyBooster(ApplyBoosterData {
            applied,
            rejected,
            timestamp,
            score,
        }));
    }

    fn get_timezone(&self, account_id: AccountId) -> Option<I64> {
        self.accounts.get(&account_id).map(|account| I64(*account.timezone))
    }

    fn get_score(&self, account_id: AccountId) -> Option<U128> {
        let account = self.get_account(&account_id);

        Some(u128::from(account.score.get_last_finalized_score(account.timezone)).into())
    }

    fn set_timezone(&mut self, account_id: AccountId, timezone: I64) {
        self.assert_manager();

        let account = self.get_or_create_account_mut(&account_id);
        account.try_set_timezone(Some(Timezone::new(timezone.0)));
    }

    fn set_feature_enabled(&mut self, account_id: AccountId, feature: Feature, enabled: bool) {
        self.assert_manager();

        if self.accounts.contains_key(&account_id) {
            self.update_account_cache(&account_id, None);
        }

        let account = self.get_or_create_account_mut(&account_id);
        account.set_feature_enabled(&feature, enabled);

        emit(EventKind::SetFeatureEnabled(account_id, feature, enabled));
    }

    fn batch_set_feature_enabled(&mut self, account_ids: Vec<AccountId>, feature: Feature, enabled: bool) {
        self.assert_manager();

        for account_id in &account_ids {
            self.update_account_cache(account_id, None);

            let account = self.get_account_mut(account_id);
            account.set_feature_enabled(&feature, enabled);
        }

        emit(EventKind::BatchSetFeatureEnabled(account_ids, feature, enabled));
    }
}
