use near_sdk::near;
use sweat_jar_model::{TokenAmount, UDecimal};

/// The `Cap` struct defines the capacity of a deposit jar in terms of the minimum and maximum allowed principal amounts.
#[near(serializers=[borsh, json])]
#[derive(Clone, Debug)]
pub struct Cap {
    /// The minimum amount of tokens that can be stored in the jar.
    pub min: TokenAmount,

    /// The maximum amount of tokens that can be stored in the jar.
    pub max: TokenAmount,
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

impl Apy {
    pub(crate) fn get_effective(&self, is_penalty_applied: bool) -> UDecimal {
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
