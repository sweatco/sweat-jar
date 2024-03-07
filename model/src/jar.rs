use std::collections::HashMap;

use near_sdk::{
    borsh,
    borsh::{BorshDeserialize, BorshSerialize},
    json_types::{U128, U64},
    serde::{Deserialize, Serialize},
    AccountId, Timestamp,
};

use crate::{numbers::U32, ProductId};

pub type JarId = u32;

pub type JarIdView = U32;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct JarView {
    pub id: JarIdView,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub created_at: U64,
    pub principal: U128,
    pub claimed_balance: U128,
    pub is_penalty_applied: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(crate = "near_sdk::serde")]
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct AggregatedInterestView {
    pub amount: AggregatedTokenAmountView,
    pub timestamp: Timestamp,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct CeFiJar {
    pub id: String,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub principal: U128,
    pub created_at: U64,
}
