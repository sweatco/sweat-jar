use near_sdk::json_types::{U128, U64};
use sweat_jar_model::product::{
    ApyView, CapView, DowngradableApyView, FixedProductTermsView, FlexibleProductTermsView, ProductView,
    ScoreBasedProductTermsView, TermsView, WithdrawalFeeView,
};

use crate::product::model::{Apy, Cap, DowngradableApy, Product, Terms, WithdrawalFee};

impl From<Product> for ProductView {
    fn from(value: Product) -> Self {
        Self {
            id: value.id,
            cap: value.cap.into(),
            terms: value.terms.into(),
            withdrawal_fee: value.withdrawal_fee.map(Into::into),
            is_enabled: value.is_enabled,
        }
    }
}

impl From<Terms> for TermsView {
    fn from(value: Terms) -> Self {
        match value {
            Terms::Fixed(value) => TermsView::Fixed(FixedProductTermsView {
                apy: value.apy.into(),
                lockup_term: U64(value.lockup_term),
            }),
            Terms::Flexible(value) => TermsView::Flexible(FlexibleProductTermsView { apy: value.apy.into() }),
            Terms::ScoreBased(value) => TermsView::ScoreBased(ScoreBasedProductTermsView {
                lockup_term: value.lockup_term.into(),
                score_cap: value.score_cap,
            }),
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
