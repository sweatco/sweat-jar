use std::cmp;

use near_sdk::{json_types::Base64VecU8, require};
use sweat_jar_model::{
    product::{FixedProductTerms, FlexibleProductTerms, Product, ScoreBasedProductTerms, Terms, WithdrawalFee},
    ProductId, Score, ToAPY, TokenAmount, UDecimal, MS_IN_DAY, MS_IN_YEAR,
};

use crate::{
    common::{Duration, Timestamp},
    env,
    jar::{
        account::Account,
        model::{Deposit, Jar},
    },
    product::model::legacy::{ProductLegacy, Terms as TermsLegacy},
    Contract,
};

pub(crate) trait TermsApi {
    fn allows_early_withdrawal(&self) -> bool;
    fn is_liquid(&self, deposit: &Deposit) -> bool;
}

impl TermsApi for Terms {
    fn allows_early_withdrawal(&self) -> bool {
        matches!(self, Terms::Flexible(_))
    }

    fn is_liquid(&self, deposit: &Deposit) -> bool {
        let now = env::block_timestamp_ms();
        match self {
            Terms::Fixed(terms) => deposit.is_liquid(now, terms.lockup_term.0),
            Terms::Flexible(_) => true,
            Terms::ScoreBased(terms) => deposit.is_liquid(now, terms.lockup_term.0),
        }
    }
}

pub(crate) trait ProductModelApi {
    fn get_public_key(self) -> Option<Vec<u8>>;
    fn set_public_key(&mut self, public_key: Option<Base64VecU8>);
    fn calculate_fee(&self, principal: TokenAmount) -> TokenAmount;
}

impl ProductModelApi for Product {
    fn get_public_key(self) -> Option<Vec<u8>> {
        self.public_key.map(|key| key.0)
    }

    fn set_public_key(&mut self, public_key: Option<Base64VecU8>) {
        self.public_key = public_key.map(Into::into);
    }

    fn calculate_fee(&self, principal: TokenAmount) -> TokenAmount {
        if let Some(fee) = self.withdrawal_fee.clone() {
            return match fee {
                WithdrawalFee::Fix(amount) => amount.0,
                WithdrawalFee::Percent(percent) => percent * principal,
            };
        }

        0
    }
}

pub(crate) trait ProductAssertions {
    fn assert_cap_order(&self);
    fn assert_cap(&self, amount: TokenAmount);
    fn assert_enabled(&self);
    fn assert_fee_amount(&self);
    fn assert_score_based_product_is_protected(&self);
}

impl ProductAssertions for Product {
    fn assert_cap_order(&self) {
        require!(self.cap.min() < self.cap.max(), "Cap minimum must be less than maximum");
    }

    fn assert_cap(&self, amount: TokenAmount) {
        if self.cap.min() > amount || amount > self.cap.max() {
            env::panic_str(&format!(
                "Total amount is out of product bounds: [{}..{}]",
                self.cap.min(),
                self.cap.max()
            ));
        }
    }

    fn assert_enabled(&self) {
        require!(
            self.is_enabled,
            "It's not possible to create new jars for this product: the product is disabled."
        );
    }

    /// Check if fee in new product is not too high
    fn assert_fee_amount(&self) {
        let Some(ref fee) = self.withdrawal_fee else {
            return;
        };

        let fee_ok = match fee {
            WithdrawalFee::Fix(amount) => amount.0 < self.cap.min(),
            WithdrawalFee::Percent(percent) => percent.to_f32() < 100.0,
        };

        require!(
            fee_ok,
            "Fee for this product is too high. It is possible for a user to pay more in fees than they staked."
        );
    }

    fn assert_score_based_product_is_protected(&self) {
        if matches!(self.terms, Terms::ScoreBased(_)) {
            require!(self.public_key.is_some(), "Score based must be protected.");
        }
    }
}

// TODO: add tests
pub(crate) trait InterestCalculator {
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

impl Contract {
    // UnorderedMap doesn't have cache and deserializes `Product` on each get
    // This cached getter significantly reduces gas usage
    #[cfg(not(test))]
    pub(crate) fn get_product(&self, product_id: &ProductId) -> Product {
        self.products_cache
            .borrow_mut()
            .entry(product_id.clone())
            .or_insert_with(|| {
                self.products
                    .get(product_id)
                    .unwrap_or_else(|| env::panic_str(format!("Product {product_id} is not found").as_str()))
            })
            .clone()
    }

    // We should avoid this caching behaviour in tests though
    #[cfg(test)]
    pub(crate) fn get_product(&self, product_id: &ProductId) -> Product {
        self.products
            .get(product_id)
            .unwrap_or_else(|| env::panic_str(format!("Product {product_id} is not found").as_str()))
    }
}

impl From<ProductLegacy> for Product {
    fn from(value: ProductLegacy) -> Self {
        let (terms, is_restakable): (Terms, bool) = match value.terms {
            TermsLegacy::Fixed(terms) => (
                if value.score_cap > 0 {
                    Terms::ScoreBased(ScoreBasedProductTerms {
                        lockup_term: terms.lockup_term.into(),
                        score_cap: value.score_cap,
                    })
                } else {
                    Terms::Fixed(FixedProductTerms {
                        lockup_term: terms.lockup_term.into(),
                        apy: value.apy,
                    })
                },
                terms.allows_restaking,
            ),
            TermsLegacy::Flexible => (Terms::Flexible(FlexibleProductTerms { apy: value.apy }), true),
        };

        Self {
            id: value.id,
            cap: value.cap,
            terms,
            withdrawal_fee: value.withdrawal_fee,
            public_key: value.public_key.map(Into::into),
            is_enabled: value.is_enabled,
            is_restakable,
        }
    }
}
