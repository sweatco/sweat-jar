use near_sdk::{
    json_types::{Base64VecU8, U128, U64},
    near,
};

use crate::{ProductId, Score, MS_IN_YEAR};

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
pub struct ProductView {
    pub id: ProductId,
    pub cap: CapView,
    pub terms: TermsView,
    pub withdrawal_fee: Option<WithdrawalFeeView>,
    pub is_enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
pub struct DowngradableApyView {
    pub default: f32,
    pub fallback: f32,
}

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
pub enum ApyView {
    Constant(f32),
    Downgradable(DowngradableApyView),
}

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
pub struct CapView {
    pub min: U128,
    pub max: U128,
}

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum TermsView {
    Fixed(FixedProductTermsView),
    Flexible(FlexibleProductTermsView),
    ScoreBased(ScoreBasedProductTermsView),
}

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
pub struct FixedProductTermsView {
    pub apy: ApyView,
    pub lockup_term: U64,
}

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
pub struct FlexibleProductTermsView {
    pub apy: ApyView,
}

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
pub struct ScoreBasedProductTermsView {
    pub base_apy: ApyView,
    pub lockup_term: U64,
    pub score_cap: Score,
}

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum WithdrawalFeeView {
    Fix(U128),
    Percent(f32),
}

#[near(serializers=[borsh, json])]
#[derive(PartialEq, Clone, Debug)]
// TODO: doc change
pub struct ProductDto {
    pub id: ProductId,
    pub cap: (U128, U128),
    pub terms: TermsDto,
    pub withdrawal_fee: Option<WithdrawalFeeDto>,
    pub public_key: Option<Base64VecU8>,
    pub is_enabled: bool,
}

#[near(serializers=[borsh, json])]
#[derive(PartialEq, Clone, Debug)]
pub struct ApyDto {
    pub default: (U128, u32),
    pub fallback: Option<(U128, u32)>,
}

#[near(serializers=[borsh, json])]
#[derive(PartialEq, Clone, Debug)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum TermsDto {
    Fixed(FixedProductTermsDto),
    Flexible(FlexibleProductTermsDto),
    ScoreBased(ScoreBasedProductTermsDto),
}

#[near(serializers=[borsh, json])]
#[derive(PartialEq, Clone, Debug)]
pub struct FixedProductTermsDto {
    pub apy: ApyDto,
    pub lockup_term: U64,
}

#[near(serializers=[borsh, json])]
#[derive(PartialEq, Clone, Debug)]
pub struct FlexibleProductTermsDto {
    pub apy: ApyDto,
}

#[near(serializers=[borsh, json])]
#[derive(PartialEq, Clone, Debug)]
pub struct ScoreBasedProductTermsDto {
    pub base_apy: ApyDto,
    pub lockup_term: U64,
    pub score_cap: Score,
}

impl Default for ProductDto {
    fn default() -> Self {
        Self {
            id: "default_product".to_string(),
            cap: (U128(100), U128(100_000_000_000)),
            terms: TermsDto::default(),
            withdrawal_fee: None,
            public_key: None,
            is_enabled: true,
        }
    }
}

// TODO: move to tests
impl Default for FixedProductTermsDto {
    fn default() -> Self {
        Self {
            lockup_term: U64(MS_IN_YEAR),
            apy: ApyDto::default(),
        }
    }
}

impl Default for ApyDto {
    fn default() -> Self {
        ApyDto {
            default: (U128(12), 2),
            fallback: None,
        }
    }
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

// TODO: move to tests
impl ProductView {
    pub fn get_base_apy(&self) -> &ApyView {
        match &self.terms {
            TermsView::Fixed(value) => &value.apy,
            TermsView::Flexible(value) => &value.apy,
            TermsView::ScoreBased(value) => &value.base_apy,
        }
    }
}
