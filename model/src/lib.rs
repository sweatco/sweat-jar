pub mod api;
pub mod claimed_amount_view;
pub mod jar;
mod numbers;
pub mod product;
pub mod withdraw;

pub use crate::numbers::U32;

pub type ProductId = String;

/// Amount of fungible tokens
pub type TokenAmount = u128;

pub const MS_IN_SECOND: u64 = 1000;
pub const MS_IN_MINUTE: u64 = MS_IN_SECOND * 60;
pub const MINUTES_IN_YEAR: u64 = 365 * 24 * 60;
pub const MS_IN_YEAR: u64 = MINUTES_IN_YEAR * MS_IN_MINUTE;
