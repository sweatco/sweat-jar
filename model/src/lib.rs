pub mod api;
pub mod claimed_amount_view;
pub mod jar;
mod numbers;
pub mod product;
mod score;
mod timezone;
mod udecimal;
pub mod withdraw;

pub use numbers::U32;
pub use score::*;
pub use timezone::*;
pub use udecimal::*;

pub type ProductId = String;

/// Amount of fungible tokens
pub type TokenAmount = u128;

/// Milliseconds since the Unix epoch (January 1, 1970 (midnight UTC/GMT))
pub type Timestamp = u64;

/// Duration in milliseconds
pub type Duration = u64;

pub const MS_IN_SECOND: u64 = 1000;
pub const MS_IN_MINUTE: u64 = MS_IN_SECOND * 60;
pub const MS_IN_HOUR: u64 = MS_IN_MINUTE * 60;
pub const MS_IN_DAY: u64 = MS_IN_HOUR * 24;
pub const MS_IN_YEAR: u64 = MS_IN_DAY * 365;
