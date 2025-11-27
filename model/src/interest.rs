use std::cmp;

use crate::{
    data::{
        account::{common::FeaturesAccess, features::Feature, Account},
        jar::{Deposit, Jar},
        product::{
            Apy, FixedProductTerms, FlexibleProductTerms, ScoreBasedProductTerms, Terms, TieredScoreBasedProductTerms,
        },
    },
    ms_in_day, ms_in_year, ConfigurableValue, Duration, Timestamp, ToAPY, TokenAmount, UDecimal, DAYS_STORED,
};

pub trait InterestCalculator {
    fn get_interest(&self, account: &Account, jar: &Jar, now: Timestamp) -> (TokenAmount, u64) {
        let since_date = jar.cache.map(|cache| cache.updated_at);
        let cached_interest = jar.cache.map_or(0, |cache| cache.interest);

        let (interest, remainder): (TokenAmount, u64) = self.get_interest_internal(account, jar, since_date, now);

        let year_ms = ms_in_year();
        let total_remainder = jar.claim_remainder + remainder;
        let remainder: u64 = total_remainder % year_ms;
        let extra_interest = u128::from(total_remainder / year_ms);

        (cached_interest + interest + extra_interest, remainder)
    }

    fn get_interest_internal(
        &self,
        account: &Account,
        jar: &Jar,
        since_date: Option<Timestamp>,
        now: Timestamp,
    ) -> (TokenAmount, u64);
}

impl InterestCalculator for Terms {
    fn get_interest_internal(
        &self,
        account: &Account,
        jar: &Jar,
        since_date: Option<Timestamp>,
        now: Timestamp,
    ) -> (TokenAmount, u64) {
        match self {
            Terms::Fixed(t) => t.get_interest_internal(account, jar, since_date, now),
            Terms::Flexible(t) => t.get_interest_internal(account, jar, since_date, now),
            Terms::ScoreBased(t) => t.get_interest_internal(account, jar, since_date, now),
            Terms::TieredScoreBased(t) => t.get_interest_internal(account, jar, since_date, now),
        }
    }
}

macro_rules! impl_fixed_apy_interest_calculator {
    ($($t:ty),*) => {
        $(
            impl InterestCalculator for $t {
                fn get_interest_internal(
                    &self,
                    account: &Account,
                    jar: &Jar,
                    since_date: Option<Timestamp>,
                    now: Timestamp,
                ) -> (TokenAmount, u64) {
                    let apy = self.get_effective_apy(account);

                    jar.deposits
                        .iter()
                        .map(|deposit| {
                            let term = self.get_interest_calculation_term(account, now, since_date, deposit);
                            if term > 0 {
                                get_interest(deposit.principal, apy, term)
                            } else {
                                (0, 0)
                            }
                        })
                        .fold((0, 0), |acc, (interest, remainder)| {
                            (acc.0 + interest, acc.1 + remainder)
                        })
                }
            }
        )*
    }
}

impl_fixed_apy_interest_calculator!(FixedProductTerms);
impl_fixed_apy_interest_calculator!(FlexibleProductTerms);

macro_rules! impl_score_based_apy_interest_calculator {
    ($($t:ty),*) => {
        $(
            impl InterestCalculator for $t {
                fn get_interest_internal(
                    &self,
                    account: &Account,
                    jar: &Jar,
                    since_date: Option<Timestamp>,
                    now: Timestamp,
                ) -> (TokenAmount, u64) {
                    jar.deposits
                        .iter()
                        .map(|deposit| {
                            let term = self.get_interest_calculation_term(account, now, since_date, deposit);
                            if term > 0 {
                                let apy = self.get_apy(account, deposit);

                                get_interest(deposit.principal, apy, term)
                            } else {
                                (0, 0)
                            }
                        })
                        .fold((0, 0), |acc, (interest, remainder)| {
                            (acc.0 + interest, acc.1 + remainder)
                        })
                }
            }
        )*
    }
}

impl_score_based_apy_interest_calculator!(ScoreBasedProductTerms);
impl_score_based_apy_interest_calculator!(TieredScoreBasedProductTerms);

pub trait FixedApyEvaluator {
    fn get_effective_apy(&self, account: &Account) -> UDecimal {
        self.get_apy()
            .get_effective(account.is_feature_enabled(&Feature::IncreasedApy))
    }

    fn get_apy(&self) -> Apy;
}

macro_rules! impl_fixed_apy_evaluator {
    ($($t:ty),*) => {
        $(
            impl FixedApyEvaluator for $t {
                fn get_apy(&self) -> Apy {
                    self.apy
                }
            }
        )*
    }
}

impl_fixed_apy_evaluator!(FixedProductTerms);
impl_fixed_apy_evaluator!(FlexibleProductTerms);

trait ScoreBasedApyEvaluator {
    fn get_apy(&self, account: &Account, deposit: &Deposit) -> UDecimal;
}

impl ScoreBasedApyEvaluator for ScoreBasedProductTerms {
    fn get_apy(&self, account: &Account, deposit: &Deposit) -> UDecimal {
        if deposit.created_at <= account.score.updated_at() {
            account.score.get_capped_pending_score(account.timezone, self.score_cap)
        } else {
            account
                .score
                .get_capped_total_finalized_score(account.timezone, self.score_cap)
        }
        .to_apy()
    }
}

impl ScoreBasedApyEvaluator for TieredScoreBasedProductTerms {
    fn get_apy(&self, account: &Account, deposit: &Deposit) -> UDecimal {
        let cap = match self.score_cap {
            ConfigurableValue::Constant(value) => value,
            ConfigurableValue::Tier(value) => {
                if account.features.is_feature_enabled(&Feature::IncreasedScoreCap) {
                    value.default
                } else {
                    value.fallback
                }
            }
        };

        let (pending_score, booster) = if deposit.created_at <= account.score.updated_at() {
            (
                account.score.get_capped_pending_score(account.timezone, cap),
                account.score.get_pending_finalized_boosters(account.timezone),
            )
        } else {
            (
                account.score.get_capped_total_finalized_score(account.timezone, cap),
                account.score.get_finalized_boosters(account.timezone),
            )
        };

        let total_score: u32 = u32::from(pending_score) + u32::from(booster);

        total_score.min(100_000).to_apy()
    }
}

trait TermEvaluator {
    fn get_interest_calculation_term(
        &self,
        account: &Account,
        now: Timestamp,
        since_date: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration;
}

impl TermEvaluator for FixedProductTerms {
    fn get_interest_calculation_term(
        &self,
        _account: &Account,
        now: Timestamp,
        since_date: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration {
        let since_date = since_date.map_or(deposit.created_at, |cache_date| {
            cmp::max(cache_date, deposit.created_at)
        });
        let until_date = cmp::min(now, deposit.created_at + self.lockup_term.0);

        until_date.saturating_sub(since_date)
    }
}

impl TermEvaluator for FlexibleProductTerms {
    fn get_interest_calculation_term(
        &self,
        _account: &Account,
        now: Timestamp,
        since_date: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration {
        let since_date = since_date.map_or(deposit.created_at, |cache_date| {
            cmp::max(cache_date, deposit.created_at)
        });

        now.saturating_sub(since_date)
    }
}

macro_rules! impl_term_evaluator_for_score_based_product_terms {
    ($($t:ty),*) => {
        $(
            impl TermEvaluator for $t {
                fn get_interest_calculation_term(
                    &self,
                    account: &Account,
                    now: Timestamp,
                    since_date: Option<Timestamp>,
                    deposit: &Deposit,
                ) -> Duration {
                    let score_update_date = account.score.updated_at();
                    let deposit_date = deposit.created_at;

                    if score_update_date < since_date.unwrap_or_default() {
                        return 0;
                    }

                    if score_update_date < deposit_date && deposit_date.abs_diff(score_update_date) > DAYS_STORED as u64 * ms_in_day() {
                        return 0;
                    }

                    let term_end = cmp::max(now, deposit_date + self.lockup_term.0);
                    if now >= term_end {
                        return 0;
                    }

                    ms_in_day()
                }
            }
        )*
    }
}

impl_term_evaluator_for_score_based_product_terms!(ScoreBasedProductTerms);
impl_term_evaluator_for_score_based_product_terms!(TieredScoreBasedProductTerms);

fn get_interest(principal: TokenAmount, apy: UDecimal, term: Duration) -> (TokenAmount, u64) {
    let year_ms: u128 = ms_in_year().into();
    let term_in_milliseconds: u128 = term.into();

    let yearly_interest = apy * principal;
    let interest = term_in_milliseconds * yearly_interest;

    // This will never fail because `MS_IN_YEAR` is u64
    // and remainder from u64 cannot be bigger than u64 so it is safe to unwrap here.
    let remainder: u64 = (interest % year_ms).try_into().unwrap();
    let interest = interest / year_ms;

    (interest, remainder)
}
