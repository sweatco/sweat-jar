use std::collections::HashMap;

use near_sdk::{
    json_types::{U128, U64},
    near, AccountId, Timestamp,
};

use crate::{numbers::U32, ProductId};

pub type JarId = u32;

pub type JarIdView = U32;

#[derive(Clone, Debug, PartialEq)]
#[near(serializers=[json])]
pub struct JarView {
    pub id: JarIdView,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: U64,
    pub principal: U128,
    pub claimed_balance: U128,
    pub is_penalty_applied: bool,
    #[serde(default)]
    pub is_pending_withdraw: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[near(serializers=[json])]
pub struct AggregatedTokenAmountView {
    pub detailed: HashMap<JarIdView, U128>,
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

#[derive(Debug, PartialEq)]
#[near(serializers=[json])]
pub struct AggregatedInterestView {
    pub amount: AggregatedTokenAmountView,
    pub timestamp: Timestamp,
}

#[near(serializers=[json])]
pub struct CeFiJar {
    pub id: String,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub principal: U128,
    pub created_at: U64,
}
