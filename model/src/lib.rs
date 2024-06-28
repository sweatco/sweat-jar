pub mod api;
pub mod claimed_amount_view;
pub mod jar;
mod numbers;
pub mod product;
pub mod withdraw;

use near_sdk::{env::block_timestamp_ms, near, Timestamp};

pub use crate::numbers::U32;

pub type ProductId = String;

/// Amount of fungible tokens
pub type TokenAmount = u128;

pub type Score = u16;

#[near]
#[derive(Copy, Clone, Debug)]
pub struct AccountScore {
    pub timezone: Timestamp,
    pub last_update: Timestamp,
    pub score: Score,
}

impl AccountScore {
    pub fn new(timezone: Timestamp) -> Self {
        Self {
            timezone,
            last_update: block_timestamp_ms(),
            score: 0,
        }
    }
}

pub const MS_IN_SECOND: u64 = 1000;
pub const MS_IN_MINUTE: u64 = MS_IN_SECOND * 60;
pub const MS_IN_HOUR: u64 = MS_IN_MINUTE * 60;
pub const MS_IN_DAY: u64 = MS_IN_HOUR * 24;
pub const MS_IN_YEAR: u64 = MS_IN_DAY * 365;
