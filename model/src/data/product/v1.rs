use near_sdk::{
    json_types::{Base64VecU8, U128, U64},
    near,
};

use crate::{ConfigurableValue, Duration, Score, TokenAmount, UDecimal};

pub type ProductId = String;

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
    pub public_key: Option<Base64VecU8>, // TODO: remove pub

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

    /// TODO: doc
    TieredScoreBased(TieredScoreBasedProductTerms),
}

/// The `FixedProductTerms` struct contains terms specific to Fixed products.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct FixedProductTerms {
    /// The maturity term of the jar in seconds, during which it yields interest.
    /// After this period, the user can withdraw principal or potentially restake the jar.
    pub lockup_term: U64,
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
    /// The maturity term of the jar in milliseconds, during which it yields interest.
    /// After this period, the user can withdraw principal or potentially restake the jar.
    pub lockup_term: U64,
}

/// TODO: doc
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
pub struct TieredScoreBasedProductTerms {
    pub score_cap: ConfigurableValue<Score>,
    /// The maturity term of the jar in milliseconds, during which it yields interest.
    /// After this period, the user can withdraw principal or potentially restake the jar.
    pub lockup_term: U64,
}

/// The `Cap` struct defines the capacity of a deposit jar in terms of the minimum and maximum allowed principal amounts.
/// - `.0` – The minimum amount of tokens that can be stored in the jar.
/// - `.1` – The maximum amount of tokens that can be stored in the jar.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug)]
pub struct Cap(U128, U128);

/// The `WithdrawalFee` enum describes withdrawal fee details, which can be either a fixed amount or a percentage of the withdrawal.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum WithdrawalFee {
    /// Describes a fixed amount of tokens that a user must pay as a fee on withdrawal.
    Fix(U128),

    /// Describes a percentage of the withdrawal amount that a user must pay as a fee on withdrawal.
    Percent(UDecimal),
}

/// The `Apy` enum describes the Annual Percentage Yield (APY) of the product, which can be either constant or downgradable.
pub type Apy = ConfigurableValue<UDecimal>;

impl Product {
    pub fn is_protected(&self) -> bool {
        self.public_key.is_some()
    }
}

impl Apy {
    pub fn get_effective(&self, is_increased_apy_enabled: bool) -> UDecimal {
        match self {
            Apy::Constant(apy) => *apy,
            Apy::Tier(apy) => {
                if is_increased_apy_enabled {
                    apy.default
                } else {
                    apy.fallback
                }
            }
        }
    }
}

impl Cap {
    pub fn new(min: TokenAmount, max: TokenAmount) -> Self {
        Self(min.into(), max.into())
    }

    pub fn min(&self) -> TokenAmount {
        self.0 .0
    }

    pub fn max(&self) -> TokenAmount {
        self.1 .0
    }
}

impl Terms {
    pub fn get_lockup_term(&self) -> Option<Duration> {
        match self {
            Terms::Fixed(terms) => Some(terms.lockup_term.0),
            Terms::Flexible(_) => None,
            Terms::ScoreBased(terms) => Some(terms.lockup_term.0),
            Terms::TieredScoreBased(terms) => Some(terms.lockup_term.0),
        }
    }

    pub fn is_score_based(&self) -> bool {
        matches!(self, Terms::ScoreBased(_) | Terms::TieredScoreBased(_))
    }
}
