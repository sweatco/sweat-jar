use near_sdk::{
    json_types::{Base64VecU8, U128, U64},
    near,
    serde::{Deserialize, Serialize},
};

use crate::{ProductId, MS_IN_YEAR};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct DowngradableApyView {
    pub default: f32,
    pub fallback: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum ApyView {
    Constant(f32),
    Downgradable(DowngradableApyView),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct CapView {
    pub min: U128,
    pub max: U128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct FixedProductTermsView {
    pub lockup_term: U64,
    pub allows_top_up: bool,
    pub allows_restaking: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum TermsView {
    Fixed(FixedProductTermsView),
    Flexible,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum WithdrawalFeeView {
    Fix(U128),
    Percent(f32),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct ProductView {
    pub id: ProductId,
    pub apy: ApyView,
    pub cap: CapView,
    pub terms: TermsView,
    pub withdrawal_fee: Option<WithdrawalFeeView>,
    pub is_enabled: bool,
}

#[near(serializers=[borsh, json])]
#[derive(PartialEq, Clone, Debug)]
pub struct FixedProductTermsDto {
    pub lockup_term: U64,
    pub allows_top_up: bool,
    pub allows_restaking: bool,
}

impl Default for FixedProductTermsDto {
    fn default() -> Self {
        Self {
            lockup_term: U64(MS_IN_YEAR),
            allows_restaking: false,
            allows_top_up: false,
        }
    }
}

#[near(serializers=[borsh, json])]
#[derive(PartialEq, Clone, Debug)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum TermsDto {
    Fixed(FixedProductTermsDto),
    Flexible,
}

impl Default for TermsDto {
    fn default() -> Self {
        Self::Fixed(FixedProductTermsDto::default())
    }
}

#[near(serializers=[borsh, json])]
#[derive(Clone, PartialEq, Debug)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum WithdrawalFeeDto {
    /// Fixed amount of tokens which a user will pay on tokens withdraw
    Fix(U128),
    /// Decimal representation of a percent that a user will pay on tokens withdraw:
    /// 1. First element is significand as a string
    /// 2. Second element is exponent as an integer
    /// I.e. "0.12" becomes ("12", 2): 12 * 10^-2
    Percent(U128, u32),
}

#[near(serializers=[borsh, json])]
#[derive(PartialEq, Clone, Debug)]
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
