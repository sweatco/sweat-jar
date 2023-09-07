use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::{U128, U64},
    serde::{Deserialize, Serialize},
    AccountId,
};

use crate::product::model::ProductId;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct CeFiJar {
    pub id: String,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub principal: U128,
    pub created_at: U64,
}
