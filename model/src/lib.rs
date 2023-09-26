pub mod jar_view;
mod u32;
pub mod withdraw_view;

use near_sdk::{
    serde::{Deserialize, Serialize},
    AccountId,
};

pub use crate::u32::U32;

pub type ProductId = String;

/// Amount of fungible tokens
pub type TokenAmount = u128;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Fee {
    pub beneficiary_id: AccountId,
    pub amount: TokenAmount,
}
