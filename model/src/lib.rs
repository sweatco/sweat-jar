pub mod api;
pub mod jar;
mod numbers;
pub mod product;
pub mod withdraw;

use std::collections::HashMap;

use near_sdk::{
    json_types::U128,
    serde::{Deserialize, Serialize},
};

use crate::jar::JarIdView;
pub use crate::numbers::U32;

pub type ProductId = String;

/// Amount of fungible tokens
pub type TokenAmount = u128;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AggregatedTokenAmountView {
    pub detailed: HashMap<JarIdView, U128>,
    pub total: U128,
}

impl Default for AggregatedTokenAmountView {
    fn default() -> Self {
        Self {
            detailed: HashMap::default(),
            total: U128(0),
        }
    }
}

pub const MS_IN_SECOND: u64 = 1000;
pub const MS_IN_MINUTE: u64 = MS_IN_SECOND * 60;
pub const MINUTES_IN_YEAR: u64 = 365 * 24 * 60;
pub const MS_IN_YEAR: u64 = MINUTES_IN_YEAR * MS_IN_MINUTE;
