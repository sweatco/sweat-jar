use std::cmp;

use near_sdk::{near, require};
use sweat_jar_model::{ProductId, Score, ToAPY, TokenAmount, UDecimal, MS_IN_DAY, MS_IN_YEAR};

use crate::{
    common::{Duration, Timestamp},
    env,
    jar::{
        account::v1::AccountV1,
        model::{Deposit, JarV2},
    },
    product::model::{
        common::{Apy, Cap, WithdrawalFee},
        legacy::{ProductLegacy as LegacyProduct, Terms as LegacyTerms},
    },
    Contract,
};

/// The `Product` struct describes the terms of a deposit jar. It can be of Flexible or Fixed type.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug)]
pub struct Product {
    /// The unique identifier of the product.
    pub id: ProductId,

    /// The capacity boundaries of the deposit jar, specifying the minimum and maximum principal amount.
    pub cap: Cap,

    /// The terms specific to the product, which can be either Flexible or Fixed.
    pub terms: Terms,

    /// Describes whether a withdrawal fee is applicable and, if so, its details.
    pub withdrawal_fee: Option<WithdrawalFee>,

    /// An optional ed25519 public key used for authorization to create a jar for this product.
    pub public_key: Option<Vec<u8>>,

    /// Indicates whether it's possible to create a new jar for this product.
    pub is_enabled: bool,
}

/// The `Terms` enum describes additional terms specific to either Flexible or Fixed products.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Terms {
    /// Describes additional terms for Fixed products.
    Fixed(FixedProductTerms),

    /// Describes additional terms for Flexible products.
    Flexible(FlexibleProductTerms),

    /// TODO: doc
    ScoreBased(ScoreBasedProductTerms),
}

/// The `FixedProductTerms` struct contains terms specific to Fixed products.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct FixedProductTerms {
    /// The maturity term of the jar, during which it yields interest. After this period, the user can withdraw principal
    /// or potentially restake the jar.
    pub lockup_term: Duration,
    pub apy: Apy,
}

/// TODO: doc
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct FlexibleProductTerms {
    pub apy: Apy,
}

/// TODO: doc
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreBasedProductTerms {
    pub score_cap: Score,
    pub base_apy: Apy,
    pub lockup_term: Duration,
}

impl Terms {
    pub(crate) fn allows_early_withdrawal(&self) -> bool {
        matches!(self, Terms::Flexible(_))
    }

    pub(crate) fn is_liquid(&self, deposit: &Deposit) -> bool {
        let now = env::block_timestamp_ms();
        match self {
            Terms::Fixed(terms) => deposit.is_liquid(now, terms.lockup_term),
            Terms::Flexible(_) => true,
            Terms::ScoreBased(terms) => deposit.is_liquid(now, terms.lockup_term),
        }
    }
}

impl Product {
    pub(crate) fn calculate_fee(&self, principal: TokenAmount) -> TokenAmount {
        if let Some(fee) = self.withdrawal_fee.clone() {
            return match fee {
                WithdrawalFee::Fix(amount) => amount,
                WithdrawalFee::Percent(percent) => percent * principal,
            };
        }

        0
    }

    pub(crate) fn assert_cap(&self, amount: TokenAmount) {
        if self.cap.min > amount || amount > self.cap.max {
            env::panic_str(&format!(
                "Total amount is out of product bounds: [{}..{}]",
                self.cap.min, self.cap.max
            ));
        }
    }

    pub(crate) fn assert_enabled(&self) {
        require!(self.is_enabled, "It's not possible to create new jars for this product");
    }

    /// Check if fee in new product is not too high
    pub(crate) fn assert_fee_amount(&self) {
        let Some(ref fee) = self.withdrawal_fee else {
            return;
        };

        let fee_ok = match fee {
            WithdrawalFee::Fix(amount) => amount < &self.cap.min,
            WithdrawalFee::Percent(percent) => percent.to_f32() < 100.0,
        };

        require!(
            fee_ok,
            "Fee for this product is too high. It is possible for a user to pay more in fees than they staked."
        );
    }
}

// TODO: add tests
pub(crate) trait InterestCalculator {
    fn get_interest(&self, account: &AccountV1, jar: &JarV2, now: Timestamp) -> (TokenAmount, u64) {
        let since_date = jar.cache.map(|cache| cache.updated_at);
        let apy = self.get_apy(account);
        let cached_interest = jar.cache.map_or(0, |cache| cache.interest);

        let (interest, remainder): (TokenAmount, u64) = jar
            .deposits
            .iter()
            .map(|deposit| {
                let term = self.get_interest_calculation_term(account, now, since_date, deposit);

                dbg!(term);
                dbg!(deposit.principal);
                dbg!(apy);

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

    fn get_apy(&self, account: &AccountV1) -> UDecimal;

    fn get_interest_calculation_term(
        &self,
        account: &AccountV1,
        now: Timestamp,
        last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration;
}

impl InterestCalculator for Terms {
    fn get_apy(&self, account: &AccountV1) -> UDecimal {
        match self {
            Terms::Fixed(terms) => terms.get_apy(account),
            Terms::Flexible(terms) => terms.get_apy(account),
            Terms::ScoreBased(terms) => terms.get_apy(account),
        }
    }

    fn get_interest_calculation_term(
        &self,
        account: &AccountV1,
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
    fn get_apy(&self, account: &AccountV1) -> UDecimal {
        self.apy.get_effective(account.is_penalty_applied)
    }

    fn get_interest_calculation_term(
        &self,
        _account: &AccountV1,
        now: Timestamp,
        last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration {
        let since_date = last_cached_at.map_or(deposit.created_at, |cache_date| {
            cmp::max(cache_date, deposit.created_at)
        });
        let until_date = cmp::min(now, deposit.created_at + self.lockup_term);

        until_date.saturating_sub(since_date)
    }
}

impl InterestCalculator for FlexibleProductTerms {
    fn get_apy(&self, account: &AccountV1) -> UDecimal {
        self.apy.get_effective(account.is_penalty_applied)
    }

    fn get_interest_calculation_term(
        &self,
        _account: &AccountV1,
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
    fn get_apy(&self, account: &AccountV1) -> UDecimal {
        let score = account.score.claimable_score();

        let total_score: Score = score.iter().map(|score| score.min(&self.score_cap)).sum();

        self.base_apy.get_effective(account.is_penalty_applied) + total_score.to_apy()
    }

    fn get_interest_calculation_term(
        &self,
        _account: &AccountV1,
        now: Timestamp,
        _last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Timestamp {
        let term_end = cmp::max(now, deposit.created_at + self.lockup_term);
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

impl From<LegacyProduct> for Product {
    fn from(value: LegacyProduct) -> Self {
        let terms: Terms = match value.terms {
            LegacyTerms::Fixed(terms) => Terms::Fixed(FixedProductTerms {
                lockup_term: terms.lockup_term,
                apy: value.apy,
            }),
            LegacyTerms::Flexible => Terms::Flexible(FlexibleProductTerms { apy: value.apy }),
        };

        Self {
            id: value.id,
            cap: Cap {
                min: value.cap.min,
                max: value.cap.max,
            },
            terms,
            withdrawal_fee: value.withdrawal_fee,
            public_key: value.public_key,
            is_enabled: value.is_enabled,
        }
    }
}
