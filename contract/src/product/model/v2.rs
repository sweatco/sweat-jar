use std::cmp;

use near_sdk::{near, require};
use sweat_jar_model::{ProductId, Score, ToAPY, TokenAmount, UDecimal, MS_IN_YEAR};

use crate::{
    common::{Duration, Timestamp},
    env,
    jar::{
        account::v2::AccountV2,
        model::{Deposit, JarV2},
    },
    Contract,
};

/// The `Product` struct describes the terms of a deposit jar. It can be of Flexible or Fixed type.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug)]
pub struct ProductV2 {
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

/// The `WithdrawalFee` enum describes withdrawal fee details, which can be either a fixed amount or a percentage of the withdrawal.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum WithdrawalFee {
    /// Describes a fixed amount of tokens that a user must pay as a fee on withdrawal.
    Fix(TokenAmount),

    /// Describes a percentage of the withdrawal amount that a user must pay as a fee on withdrawal.
    Percent(UDecimal),
}

/// The `Apy` enum describes the Annual Percentage Yield (APY) of the product, which can be either constant or downgradable.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Apy {
    /// Describes a constant APY, where the interest remains the same throughout the product's term.
    Constant(UDecimal),

    /// Describes a downgradable APY, where an oracle can set a penalty if a user violates the product's terms.
    Downgradable(DowngradableApy),
}

/// The `DowngradableApy` struct describes an APY that can be downgraded by an oracle.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct DowngradableApy {
    /// The default APY value if the user meets all the terms of the product.
    pub default: UDecimal,

    /// The fallback APY value if the user violates some of the terms of the product.
    pub fallback: UDecimal,
}

/// The `Cap` struct defines the capacity of a deposit jar in terms of the minimum and maximum allowed principal amounts.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug)]
pub struct Cap {
    /// The minimum amount of tokens that can be stored in the jar.
    pub min: TokenAmount,

    /// The maximum amount of tokens that can be stored in the jar.
    pub max: TokenAmount,
}

impl Terms {
    pub(crate) fn allows_early_withdrawal(&self) -> bool {
        match self {
            Terms::Flexible(_) => true,
            _ => false,
        }
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

impl ProductV2 {
    pub(crate) fn calculate_fee(&self, principal: TokenAmount) -> TokenAmount {
        if let Some(fee) = self.withdrawal_fee.clone() {
            return match fee {
                WithdrawalFee::Fix(amount) => amount.clone(),
                WithdrawalFee::Percent(percent) => percent * principal,
            };
        }

        0
    }

    // TODO: should it test total principal?
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

    /// Check if fee in new product is not to high
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

pub(crate) trait InterestCalculator {
    fn get_interest(&self, account: &AccountV2, jar: &JarV2, now: Timestamp) -> (TokenAmount, u64) {
        let since_date = jar.cache.map(|cache| cache.updated_at);
        let apy = self.get_apy(account);

        let (interest, remainder): (TokenAmount, u64) = jar
            .deposits
            .iter()
            .map(|deposit| {
                let term = self.get_interest_calculation_term(now, since_date, deposit);

                if term > 0 {
                    get_interest(deposit.principal, apy, term)
                } else {
                    (0, 0)
                }
            })
            .fold((0, 0), |acc, (interest, remainder)| {
                (acc.0 + interest, acc.1 + remainder)
            });

        let remainder: u64 = remainder % MS_IN_YEAR;
        let extra_interest = (remainder / MS_IN_YEAR) as u128;

        (interest + extra_interest, remainder)
    }

    fn get_apy(&self, account: &AccountV2) -> UDecimal;

    fn get_interest_calculation_term(
        &self,
        now: Timestamp,
        last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration;
}

impl InterestCalculator for Terms {
    fn get_apy(&self, account: &AccountV2) -> UDecimal {
        match self {
            Terms::Fixed(terms) => terms.get_apy(account),
            Terms::Flexible(terms) => terms.get_apy(account),
            Terms::ScoreBased(terms) => terms.get_apy(account),
        }
    }

    fn get_interest_calculation_term(
        &self,
        now: Timestamp,
        last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration {
        match self {
            Terms::Fixed(terms) => terms.get_interest_calculation_term(now, last_cached_at, deposit),
            Terms::Flexible(terms) => terms.get_interest_calculation_term(now, last_cached_at, deposit),
            Terms::ScoreBased(terms) => terms.get_interest_calculation_term(now, last_cached_at, deposit),
        }
    }
}

impl InterestCalculator for FixedProductTerms {
    fn get_apy(&self, account: &AccountV2) -> UDecimal {
        self.apy.get_effective(account.is_penalty_applied)
    }

    fn get_interest_calculation_term(
        &self,
        now: Timestamp,
        last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Duration {
        let since_date = last_cached_at.map_or(deposit.created_at, |cache_date| {
            cmp::max(cache_date, deposit.created_at)
        });
        let until_date = cmp::min(now, deposit.created_at + self.lockup_term);

        until_date.checked_sub(since_date).unwrap_or(0)
    }
}

impl InterestCalculator for FlexibleProductTerms {
    fn get_apy(&self, account: &AccountV2) -> UDecimal {
        self.apy.get_effective(account.is_penalty_applied)
    }

    fn get_interest_calculation_term(
        &self,
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
    fn get_apy(&self, account: &AccountV2) -> UDecimal {
        let score = account.score.claimable_score();

        let total_score: Score = score.iter().map(|score| score.min(&self.score_cap)).sum();
        self.base_apy.get_effective(account.is_penalty_applied) + total_score.to_apy()
    }

    fn get_interest_calculation_term(
        &self,
        now: Timestamp,
        last_cached_at: Option<Timestamp>,
        deposit: &Deposit,
    ) -> Timestamp {
        let since_date = last_cached_at.map_or(deposit.created_at, |cache_date| {
            cmp::max(cache_date, deposit.created_at)
        });
        let until_date = cmp::min(now, deposit.created_at + self.lockup_term);

        until_date.checked_sub(since_date).unwrap_or(0)
    }
}

fn get_interest(principal: TokenAmount, apy: UDecimal, term: Duration) -> (TokenAmount, u64) {
    let term_in_milliseconds: u128 = term.into();

    let yearly_interest = apy * principal;

    let ms_in_year: u128 = MS_IN_YEAR.into();

    let interest = term_in_milliseconds * yearly_interest;

    // This will never fail because `MS_IN_YEAR` is u64
    // and remainder from u64 cannot be bigger than u64 so it is safe to unwrap here.
    let remainder: u64 = (interest % ms_in_year).try_into().unwrap();
    let interest = interest / ms_in_year;

    (interest, remainder)
}

impl Apy {
    fn get_effective(&self, is_penalty_applied: bool) -> UDecimal {
        match self {
            Apy::Constant(apy) => apy.clone(),
            Apy::Downgradable(apy) => {
                if is_penalty_applied {
                    apy.fallback
                } else {
                    apy.default
                }
            }
        }
    }
}

impl Contract {
    // UnorderedMap doesn't have cache and deserializes `Product` on each get
    // This cached getter significantly reduces gas usage
    pub(crate) fn get_product(&self, product_id: &ProductId) -> ProductV2 {
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
}
