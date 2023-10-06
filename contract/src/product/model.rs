use model::{ProductId, TokenAmount};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    require,
    serde::{Deserialize, Serialize},
};

use crate::{
    common::{udecimal::UDecimal, Duration},
    env,
};

/// The `Product` struct describes the terms of a deposit jar. It can be of Flexible or Fixed type.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Product {
    /// The unique identifier of the product.
    pub id: ProductId,

    /// The Annual Percentage Yield (APY) associated with the product.
    pub apy: Apy,

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
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum Terms {
    /// Describes additional terms for Fixed products.
    Fixed(FixedProductTerms),

    /// Describes additional terms for Flexible products.
    Flexible,
}

/// The `FixedProductTerms` struct contains terms specific to Fixed products.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct FixedProductTerms {
    /// The maturity term of the jar, during which it yields interest. After this period, the user can withdraw principal
    /// or potentially restake the jar.
    pub lockup_term: Duration,

    /// Indicates whether a user can refill the jar.
    pub allows_top_up: bool,

    /// Indicates whether a user can restake the jar after maturity.
    pub allows_restaking: bool,
}

/// The `WithdrawalFee` enum describes withdrawal fee details, which can be either a fixed amount or a percentage of the withdrawal.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum WithdrawalFee {
    /// Describes a fixed amount of tokens that a user must pay as a fee on withdrawal.
    Fix(TokenAmount),

    /// Describes a percentage of the withdrawal amount that a user must pay as a fee on withdrawal.
    Percent(UDecimal),
}

/// The `Apy` enum describes the Annual Percentage Yield (APY) of the product, which can be either constant or downgradable.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum Apy {
    /// Describes a constant APY, where the interest remains the same throughout the product's term.
    Constant(UDecimal),

    /// Describes a downgradable APY, where an oracle can set a penalty if a user violates the product's terms.
    Downgradable(DowngradableApy),
}

/// The `DowngradableApy` struct describes an APY that can be downgraded by an oracle.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct DowngradableApy {
    /// The default APY value if the user meets all the terms of the product.
    pub default: UDecimal,

    /// The fallback APY value if the user violates some of the terms of the product.
    pub fallback: UDecimal,
}

/// The `Cap` struct defines the capacity of a deposit jar in terms of the minimum and maximum allowed principal amounts.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Cap {
    /// The minimum amount of tokens that can be stored in the jar.
    pub min: TokenAmount,

    /// The maximum amount of tokens that can be stored in the jar.
    pub max: TokenAmount,
}

impl Product {
    pub(crate) fn is_flexible(&self) -> bool {
        self.terms == Terms::Flexible
    }

    pub(crate) fn allows_top_up(&self) -> bool {
        self.is_enabled
            && match &self.terms {
                Terms::Fixed(value) => value.allows_top_up,
                Terms::Flexible => true,
            }
    }

    pub(crate) fn allows_restaking(&self) -> bool {
        match &self.terms {
            Terms::Fixed(value) => value.allows_restaking,
            Terms::Flexible => false,
        }
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
            "Fee for this product is too high. It is possible for customer to pay more in fees than he staked."
        );
    }
}

#[cfg(test)]
impl Product {
    pub(crate) fn get_lockup_term(&self) -> Option<Duration> {
        match self.clone().terms {
            Terms::Fixed(value) => Some(value.lockup_term),
            Terms::Flexible => None,
        }
    }
}
