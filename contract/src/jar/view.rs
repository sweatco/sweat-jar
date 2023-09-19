use std::{collections::HashMap, fmt::Debug};

use near_sdk::{
    json_types::{U128, U64},
    serde::{Deserialize, Serialize},
    AccountId,
};

use crate::{
    common::{u32::U32, Timestamp},
    product::model::ProductId,
    Jar,
};

pub type JarIndexView = U32;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct JarView {
    pub index: JarIndexView,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: U64,
    pub principal: U128,
    pub claimed_balance: U128,
    pub is_penalty_applied: bool,
}

impl From<Jar> for JarView {
    fn from(value: Jar) -> Self {
        Self {
            index: U32(value.index),
            account_id: value.account_id,
            product_id: value.product_id,
            created_at: U64(value.created_at),
            principal: U128(value.principal),
            claimed_balance: U128(value.claimed_balance),
            is_penalty_applied: value.is_penalty_applied,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct AggregatedTokenAmountView {
    pub detailed: HashMap<JarIndexView, U128>,
    pub total: U128,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct AggregatedInterestView {
    pub amount: AggregatedTokenAmountView,
    pub timestamp: Timestamp,
}
