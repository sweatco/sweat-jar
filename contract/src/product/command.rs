use sweat_jar_model::product::{RegisterProductCommand, TermsDto, WithdrawalFeeDto};

use crate::{
    common::udecimal::UDecimal,
    product::model::{Apy, Cap, DowngradableApy, FixedProductTerms, Product, Terms, WithdrawalFee},
};

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
