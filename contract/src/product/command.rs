use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::{Base64VecU8, U128, U64},
    serde::{Deserialize, Serialize},
};

use crate::{
    common::UDecimal,
    product::model::{Apy, Cap, DowngradableApy, FixedProductTerms, Product, ProductId, Terms, WithdrawalFee},
};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct RegisterProductCommand {
    pub id: ProductId,
    pub apy_default: (U128, u32),
    pub apy_fallback: Option<(U128, u32)>,
    pub cap_min: U128,
    pub cap_max: U128,
    pub terms: TermsDto,
    pub withdrawal_fee: Option<WithdrawalFeeDto>,
    pub public_key: Option<Base64VecU8>,
    pub is_enabled: bool,
}

#[cfg(test)]
impl Default for RegisterProductCommand {
    fn default() -> Self {
        Self {
            id: "default_product".to_string(),
            apy_default: (U128(12), 2),
            apy_fallback: None,
            cap_min: U128(100),
            cap_max: U128(100_000_000_000),
            terms: TermsDto::default(),
            withdrawal_fee: None,
            public_key: None,
            is_enabled: true,
        }
    }
}

impl From<RegisterProductCommand> for Product {
    fn from(value: RegisterProductCommand) -> Self {
        let apy = if let Some(apy_fallback) = value.apy_fallback {
            Apy::Downgradable(DowngradableApy {
                default: UDecimal::new(value.apy_default.0 .0, value.apy_default.1),
                fallback: UDecimal::new(apy_fallback.0 .0, apy_fallback.1),
            })
        } else {
            Apy::Constant(UDecimal::new(value.apy_default.0 .0, value.apy_default.1))
        };
        let withdrawal_fee = value.withdrawal_fee.map(|dto| match dto {
            WithdrawalFeeDto::Fix(value) => WithdrawalFee::Fix(value.0),
            WithdrawalFeeDto::Percent(significand, exponent) => {
                WithdrawalFee::Percent(UDecimal::new(significand.0, exponent))
            }
        });

        Self {
            id: value.id,
            apy,
            cap: Cap {
                min: value.cap_min.0,
                max: value.cap_max.0,
            },
            terms: value.terms.into(),
            withdrawal_fee,
            public_key: value.public_key.map(|key| key.0),
            is_enabled: value.is_enabled,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum TermsDto {
    Fixed(FixedProductTermsDto),
    Flexible,
}

#[cfg(test)]
impl Default for TermsDto {
    fn default() -> Self {
        Self::Fixed(Default::default())
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, PartialEq, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct FixedProductTermsDto {
    pub lockup_term: U64,
    pub allows_top_up: bool,
    pub allows_restaking: bool,
}

#[cfg(test)]
impl Default for FixedProductTermsDto {
    fn default() -> Self {
        Self {
            lockup_term: U64(crate::product::tests::YEAR_LOCKUP_TERM),
            allows_restaking: false,
            allows_top_up: false,
        }
    }
}

impl From<TermsDto> for Terms {
    fn from(value: TermsDto) -> Self {
        match value {
            TermsDto::Fixed(value) => Terms::Fixed(FixedProductTerms {
                lockup_term: value.lockup_term.0,
                allows_top_up: value.allows_top_up,
                allows_restaking: value.allows_restaking,
            }),
            TermsDto::Flexible => Terms::Flexible,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum WithdrawalFeeDto {
    /// Fixed amount of tokens which a user will pay on tokens withdraw
    Fix(U128),
    /// Decimal representation of a percent that a user will pay on tokens withdraw:
    /// 1. First element is significand as a string
    /// 2. Second element is exponent as an integer
    /// I.e. "0.12" becomes ("12", 2): 12 * 10^-2
    Percent(U128, u32),
}
