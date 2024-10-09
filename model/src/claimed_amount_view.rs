use near_sdk::{json_types::U128, near};

use crate::{
    jar::{AggregatedTokenAmountView, JarId},
    ProductId, TokenAmount, U32,
};

#[derive(Debug, PartialEq, Clone)]
#[near(serializers=[json])]
#[serde(untagged)]
pub enum ClaimedAmountView {
    Total(U128),
    Detailed(AggregatedTokenAmountView),
}

impl ClaimedAmountView {
    pub fn new(detailed: Option<bool>) -> Self {
        if detailed.unwrap_or(false) {
            Self::Detailed(AggregatedTokenAmountView::default())
        } else {
            Self::Total(U128(0))
        }
    }

    pub fn get_total(&self) -> U128 {
        match self {
            ClaimedAmountView::Total(value) => *value,
            ClaimedAmountView::Detailed(value) => value.total,
        }
    }

    pub fn add(&mut self, product_id: &ProductId, amount: TokenAmount) {
        match self {
            ClaimedAmountView::Total(value) => {
                value.0 += amount;
            }
            ClaimedAmountView::Detailed(value) => {
                value.total.0 += amount;
                value.detailed.insert(product_id.clone(), U128(amount));
            }
        }
    }
}
