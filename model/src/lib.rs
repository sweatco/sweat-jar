pub mod jar;
mod numbers;
pub mod withdraw;

pub use crate::numbers::U32;

pub type ProductId = String;

/// Amount of fungible tokens
pub type TokenAmount = u128;
