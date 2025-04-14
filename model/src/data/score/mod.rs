use near_sdk::near;

use crate::{Local, Timezone, UDecimal, UTC};

mod common;

pub use common::*;

pub const DAYS_STORED: usize = 2;

pub type Score = u16;

pub trait ToAPY {
    fn to_apy(self) -> UDecimal;
}

impl ToAPY for Score {
    /// 1000 scores = 1%
    fn to_apy(self) -> UDecimal {
        UDecimal::new(self.into(), 5)
    }
}

#[derive(Default)]
pub struct ScoreRecord {
    pub score: Vec<Score>,
    pub updated: UTC,
}

pub type Chain = Vec<(Score, Local)>;

#[near(serializers=[borsh, json])]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AccountScore {
    pub updated: UTC,
    pub timezone: Timezone,
    /// Scores buffer used for interest calculation. Can be invalidated on claim.
    pub scores: [Score; DAYS_STORED],
    /// Score history values used for displaying it in application. Will not be invalidated during claim.
    pub scores_history: [Score; DAYS_STORED],
}