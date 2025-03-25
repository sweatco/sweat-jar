use near_sdk::{
    json_types::{Base64VecU8, U128, U64},
    near,
};

use crate::{ProductId, Score, TokenAmount, UDecimal};

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

    #[deprecated(note = "It doesn't have any effect and will be removed")]
    pub is_restakable: bool,
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
    /// The maturity term of the jar in milliseconds, during which it yields interest.
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

/// The `Cap` struct defines the capacity of a deposit jar in terms of the minimum and maximum allowed principal amounts.
/// - `.0` – The minimum amount of tokens that can be stored in the jar.
/// - `.1` – The maximum amount of tokens that can be stored in the jar.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug)]
pub struct Cap(U128, U128);

impl Cap {
    pub fn min(&self) -> TokenAmount {
        self.0 .0
    }

    pub fn max(&self) -> TokenAmount {
        self.1 .0
    }
}

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
#[near(serializers=[borsh])]
#[derive(Clone, Debug, PartialEq)]
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

impl Apy {
    pub fn get_effective(&self, is_penalty_applied: bool) -> UDecimal {
        match self {
            Apy::Constant(apy) => *apy,
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

// TODO: move to test cfg
pub mod test_utils {
    use crate::{
        product::{Apy, Cap, DowngradableApy, FixedProductTerms, Product, Terms, WithdrawalFee},
        TokenAmount, UDecimal, MS_IN_YEAR,
    };

    /// Default product name. If product name wasn't specified it will have this name.
    pub const DEFAULT_PRODUCT_NAME: &str = "product";
    pub const DEFAULT_SCORE_PRODUCT_NAME: &str = "score_product";

    impl Default for Product {
        fn default() -> Self {
            Self {
                id: DEFAULT_PRODUCT_NAME.to_string(),
                cap: Cap::default(),
                terms: Terms::Fixed(FixedProductTerms {
                    lockup_term: MS_IN_YEAR.into(),
                    apy: Apy::new_downgradable(),
                }),
                withdrawal_fee: None,
                public_key: None,
                is_enabled: true,
                is_restakable: true,
            }
        }
    }

    impl Default for Cap {
        fn default() -> Self {
            Self::new(0, 1_000_000_000 * 10u128.pow(18))
        }
    }

    impl Product {
        pub fn get_base_apy(&self) -> &Apy {
            match &self.terms {
                Terms::Fixed(value) => &value.apy,
                Terms::Flexible(value) => &value.apy,
                Terms::ScoreBased(_) => panic!("No APY for a score based product"),
            }
        }

        #[must_use]
        pub fn with_id(mut self, id: &str) -> Self {
            self.id = id.to_string();
            self
        }

        #[must_use]
        pub fn with_enabled(mut self, enabled: bool) -> Self {
            self.is_enabled = enabled;
            self
        }

        #[must_use]
        pub fn with_terms(mut self, terms: Terms) -> Self {
            self.terms = terms;
            self
        }

        #[must_use]
        pub fn with_public_key(mut self, public_key: Option<Vec<u8>>) -> Self {
            self.public_key = public_key.map(Into::into);
            self
        }

        #[must_use]
        pub fn with_withdrawal_fee(mut self, fee: WithdrawalFee) -> Self {
            self.withdrawal_fee = Some(fee);
            self
        }

        #[must_use]
        pub fn with_cap(mut self, min: TokenAmount, max: TokenAmount) -> Self {
            self.cap = Cap::new(min, max);
            self
        }
    }

    impl Default for Apy {
        fn default() -> Self {
            Self::Constant(UDecimal::new(12, 2))
        }
    }

    impl Cap {
        pub fn new(min: TokenAmount, max: TokenAmount) -> Self {
            Self(min.into(), max.into())
        }
    }

    impl Apy {
        pub fn new_downgradable() -> Self {
            Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(20, 2),
                fallback: UDecimal::new(10, 2),
            })
        }
    }
}

pub mod serde_utils {
    use near_sdk::{
        near,
        serde::{Deserialize, Deserializer, Serialize, Serializer},
    };

    use crate::{
        product::{Apy, DowngradableApy},
        UDecimal,
    };

    #[near(serializers=[json])]
    struct ApyHelper {
        default: UDecimal,
        #[serde(skip_serializing_if = "Option::is_none")]
        fallback: Option<UDecimal>,
    }

    impl From<Apy> for ApyHelper {
        fn from(apy: Apy) -> Self {
            match apy {
                Apy::Constant(value) => Self {
                    default: value,
                    fallback: None,
                },
                Apy::Downgradable(value) => Self {
                    default: value.default,
                    fallback: Some(value.fallback),
                },
            }
        }
    }

    impl Serialize for Apy {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            ApyHelper::from(self.clone()).serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Apy {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let helper = ApyHelper::deserialize(deserializer)?;
            Ok(match helper.fallback {
                Some(fallback) => Apy::Downgradable(DowngradableApy {
                    default: helper.default,
                    fallback,
                }),
                None => Apy::Constant(helper.default),
            })
        }
    }
}
