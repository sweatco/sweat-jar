use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8, U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use crate::product::model::ProductId;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub struct RegisterProductCommand {
    pub id: ProductId,
    pub lockup_term: U64,
    pub apy_default: (U128, u32),
    pub apy_fallback: Option<(U128, u32)>,
    pub cap_min: U128,
    pub cap_max: U128,
    pub is_refillable: bool,
    pub is_restakable: bool,
    pub withdrawal_fee: Option<WithdrawalFee>,
    pub public_key: Option<Base64VecU8>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(PartialEq))]
pub enum WithdrawalFee {
    /// Fixed amount of tokens which a user will pay on tokens withdraw
    Fix(U128),
    /// Decimal representation of a percent that a user will pay on tokens withdraw:
    /// 1. First element is significand as a string
    /// 2. Second element is exponent as an integer
    /// I.e. "0.12" becomes ("12", 2): 12 * 10^-2
    Percent(U128, u32),
}
