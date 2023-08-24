use near_sdk::AccountId;
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};

use crate::*;
use crate::product::model::ProductId;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct JarView {
    pub index: JarIndex,
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
            index: value.index,
            account_id: value.account_id,
            product_id: value.product_id,
            created_at: U64(value.created_at),
            principal: U128(value.principal),
            claimed_balance: U128(value.claimed_balance),
            is_penalty_applied: value.is_penalty_applied,
        }
    }
}