use near_sdk::{
    json_types::U128,
    serde::{Deserialize, Serialize},
};

use crate::{
    jar::{AggregatedTokenAmountView, JarId},
    TokenAmount, U32,
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde", untagged)]
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

    pub fn add(&mut self, jar_id: JarId, amout: TokenAmount) {
        match self {
            ClaimedAmountView::Total(value) => {
                value.0 += amout;
            }
            ClaimedAmountView::Detailed(value) => {
                value.total.0 += amout;
                value.detailed.insert(U32(jar_id), U128(amout));
            }
        }
    }
}