use near_sdk::{
    json_types::{U128, U64},
    serde::{Deserialize, Serialize},
    AccountId,
};

use crate::{numbers::U32, ProductId};

pub type JarId = u32;

pub type JarIdView = U32;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
