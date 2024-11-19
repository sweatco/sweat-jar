use std::collections::HashMap;

use near_sdk::{
    json_types::{U128, U64},
    near, Timestamp,
};

use crate::ProductId;

pub type JarId = u32;

pub type JarIdView = String;

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
pub struct JarView {
    pub id: JarIdView,
    pub product_id: ProductId,
    pub created_at: U64,
    pub principal: U128,
}

#[derive(Debug, Clone, PartialEq)]
#[near(serializers=[json])]
pub struct AggregatedTokenAmountView {
    pub detailed: HashMap<ProductId, U128>,
    pub total: U128,
}

impl Default for AggregatedTokenAmountView {
    fn default() -> Self {
        Self {
            detailed: HashMap::default(),
            total: U128(0),
        }
    }
}

#[derive(Debug, PartialEq, Default)]
#[near(serializers=[json])]
pub struct AggregatedInterestView {
    pub amount: AggregatedTokenAmountView,
    pub timestamp: Timestamp,
}
