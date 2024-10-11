use near_sdk::json_types::{U128, U64};
use sweat_jar_model::product::{
    ApyView, CapView, DowngradableApyView, FixedProductTermsView, ProductView, TermsView, WithdrawalFeeView,
};

use crate::product::model::ProductV2;

impl From<ProductV2> for ProductView {
    fn from(value: ProductV2) -> Self {
        Self {
            id: value.id,
            apy: value.apy.into(),
            cap: value.cap.into(),
            terms: value.terms.into(),
            withdrawal_fee: value.withdrawal_fee.map(Into::into),
            is_enabled: value.is_enabled,
            score_cap: value.score_cap,
        }
    }
}

impl From<Terms> for TermsView {
    fn from(value: Terms) -> Self {
        match value {
            Terms::Fixed(value) => TermsView::Fixed(FixedProductTermsView {
                lockup_term: U64(value.lockup_term),
                allows_top_up: value.allows_top_up,
                allows_restaking: value.allows_restaking,
            }),
            Terms::Flexible => TermsView::Flexible,
        }
    }
}

impl From<WithdrawalFee> for WithdrawalFeeView {
    fn from(value: WithdrawalFee) -> Self {
        match value {
            WithdrawalFee::Fix(value) => WithdrawalFeeView::Fix(U128(value)),
            WithdrawalFee::Percent(value) => WithdrawalFeeView::Percent(value.to_f32()),
        }
    }
}

impl From<Apy> for ApyView {
    fn from(value: Apy) -> Self {
        match value {
            Apy::Constant(value) => ApyView::Constant(value.to_f32()),
            Apy::Downgradable(value) => ApyView::Downgradable(value.into()),
        }
    }
}

impl From<DowngradableApy> for DowngradableApyView {
    fn from(value: DowngradableApy) -> Self {
        Self {
            default: value.default.to_f32(),
            fallback: value.fallback.to_f32(),
        }
    }
}

impl From<Cap> for CapView {
    fn from(value: Cap) -> Self {
        Self {
            min: U128(value.min),
            max: U128(value.max),
        }
    }
}
