use std::cmp;

use crate::{
    data::{
        account::Account,
        jar::{Deposit, Jar},
        product::{FixedProductTerms, FlexibleProductTerms, ScoreBasedProductTerms, Terms},
    },
    Duration, Score, Timestamp, ToAPY, TokenAmount, UDecimal, MS_IN_DAY, MS_IN_YEAR,
};

// TODO: add tests
pub trait InterestCalculator {
    fn get_interest(&self, account: &Account, jar: &Jar, now: Timestamp) -> (TokenAmount, u64) {
        let since_date = jar.cache.map(|cache| cache.updated_at);
        let apy = self.get_apy(account);
        let cached_interest = jar.cache.map_or(0, |cache| cache.interest);

        let (interest, remainder): (TokenAmount, u64) = jar
            .deposits
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
            });

        let total_remainder = jar.claim_remainder + remainder;
        let remainder: u64 = total_remainder % MS_IN_YEAR;
        let extra_interest = (total_remainder / MS_IN_YEAR) as u128;

        (cached_interest + interest + extra_interest, remainder)
    }

    fn get_apy(&self, account: &Account) -> UDecimal;

    fn get_interest_calculation_term(
        &self,
        account: &Account,
        now: Timestamp,
        last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration;
}

impl InterestCalculator for Terms {
    fn get_apy(&self, account: &Account) -> UDecimal {
        match self {
            Terms::Fixed(terms) => terms.get_apy(account),
            Terms::Flexible(terms) => terms.get_apy(account),
            Terms::ScoreBased(terms) => terms.get_apy(account),
        }
    }

    fn get_interest_calculation_term(
        &self,
        account: &Account,
        now: Timestamp,
        last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration {
        match self {
            Terms::Fixed(terms) => terms.get_interest_calculation_term(account, now, last_cached_at, deposit),
            Terms::Flexible(terms) => terms.get_interest_calculation_term(account, now, last_cached_at, deposit),
            Terms::ScoreBased(terms) => terms.get_interest_calculation_term(account, now, last_cached_at, deposit),
        }
    }
}

impl InterestCalculator for FixedProductTerms {
    fn get_apy(&self, account: &Account) -> UDecimal {
        self.apy.get_effective(account.is_penalty_applied)
    }

    fn get_interest_calculation_term(
        &self,
        _account: &Account,
        now: Timestamp,
        last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration {
        let since_date = last_cached_at.map_or(deposit.created_at, |cache_date| {
            cmp::max(cache_date, deposit.created_at)
        });
        let until_date = cmp::min(now, deposit.created_at + self.lockup_term.0);

        until_date.saturating_sub(since_date)
    }
}

impl InterestCalculator for FlexibleProductTerms {
    fn get_apy(&self, account: &Account) -> UDecimal {
        self.apy.get_effective(account.is_penalty_applied)
    }

    fn get_interest_calculation_term(
        &self,
        _account: &Account,
        now: Timestamp,
        last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration {
        let since_date = last_cached_at.map_or(deposit.created_at, |cache_date| {
            cmp::max(cache_date, deposit.created_at)
        });

        now - since_date
    }
}

impl InterestCalculator for ScoreBasedProductTerms {
    fn get_apy(&self, account: &Account) -> UDecimal {
        let score = account.score.claimable_score().score;
        let total_score: Score = score.iter().map(|score| score.min(&self.score_cap)).sum();

        total_score.to_apy()
    }

    fn get_interest_calculation_term(
        &self,
        account: &Account,
        now: Timestamp,
        _last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Timestamp {
        if account.score.updated.0 < deposit.created_at {
            return 0;
        }

        let term_end = cmp::max(now, deposit.created_at + self.lockup_term.0);
        if now >= term_end {
            return 0;
        }

        MS_IN_DAY
    }
}

fn get_interest(principal: TokenAmount, apy: UDecimal, term: Duration) -> (TokenAmount, u64) {
    let ms_in_year: u128 = MS_IN_YEAR.into();
    let term_in_milliseconds: u128 = term.into();

    let yearly_interest = apy * principal;
    let interest = term_in_milliseconds * yearly_interest;

    // This will never fail because `MS_IN_YEAR` is u64
    // and remainder from u64 cannot be bigger than u64 so it is safe to unwrap here.
    let remainder: u64 = (interest % ms_in_year).try_into().unwrap();
    let interest = interest / ms_in_year;

    (interest, remainder)
}
