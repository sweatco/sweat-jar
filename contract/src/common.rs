use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

/// Milliseconds since the Unix epoch (January 1, 1970 (midnight UTC/GMT))
pub type Timestamp = u64;

/// Duration in milliseconds
pub type Duration = u64;

/// `UDecimal` represents a scientific representation of decimals.
///
/// The decimal number is represented in the form of `significand` divided by (10 raised to the power of `exponent`).
/// The `significand` and `exponent` are both positive integers.
/// The key components of this structure include:
///
/// * `significand`: The parts of the decimal number that holds significant digits, i.e., all digits including and
///                  following the leftmost nonzero digit.
///
/// * `exponent`: The part of the decimal number that represents the power to which 10 must be raised to yield the original number.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct UDecimal {
    pub significand: u128,
    pub exponent: u32,
}
