use sweat_jar_model::{
    product::{ApyDto, ProductDto, TermsDto, WithdrawalFeeDto},
    UDecimal,
};

use crate::product::model::{
    Apy, Cap, DowngradableApy, FixedProductTerms, FlexibleProductTerms, ProductV2, ScoreBasedProductTerms, Terms,
    WithdrawalFee,
};

impl From<ProductDto> for ProductV2 {
    fn from(value: ProductDto) -> Self {
        Self {
            id: value.id,
            cap: Cap {
                min: value.cap.0 .0,
                max: value.cap.1 .0,
            },
            terms: value.terms.into(),
            withdrawal_fee: value.withdrawal_fee.map(Into::into),
            public_key: value.public_key.map(|key| key.0),
            is_enabled: value.is_enabled,
        }
    }
}

impl From<TermsDto> for Terms {
    fn from(value: TermsDto) -> Self {
        match value {
            TermsDto::Fixed(value) => Terms::Fixed(FixedProductTerms {
                apy: value.apy.into(),
                lockup_term: value.lockup_term.0,
            }),
            TermsDto::Flexible(value) => Terms::Flexible(FlexibleProductTerms { apy: value.apy.into() }),
            TermsDto::ScoreBased(value) => Terms::ScoreBased(ScoreBasedProductTerms {
                score_cap: value.score_cap,
                base_apy: value.base_apy.into(),
                lockup_term: value.lockup_term.0,
            }),
        }
    }
}

impl From<ApyDto> for Apy {
    fn from(value: ApyDto) -> Self {
        match value.fallback {
            None => Apy::Constant(value.default.into()),
            Some(fallback) => Apy::Downgradable(DowngradableApy {
                default: value.default.into(),
                fallback: fallback.into(),
            }),
        }
    }
}

impl From<WithdrawalFeeDto> for WithdrawalFee {
    fn from(value: WithdrawalFeeDto) -> Self {
        match value {
            WithdrawalFeeDto::Fix(value) => WithdrawalFee::Fix(value.0),
            WithdrawalFeeDto::Percent(significand, exponent) => {
                WithdrawalFee::Percent(UDecimal::new(significand.0, exponent))
            }
        }
    }
}
