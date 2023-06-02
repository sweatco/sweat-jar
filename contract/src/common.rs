use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

/// Milliseconds since the Unix epoch (January 1, 1970 (midnight UTC/GMT))
pub type Timestamp = u64;

/// Duration in milliseconds
pub type Duration = u64;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct UDecimal {
    pub significand: u128,
    pub exponent: u32,
}
