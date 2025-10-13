use near_sdk::env::block_timestamp_ms;

use super::{DailyScore, DAYS_STORED};
use crate::{AccountScore, Score};

impl AccountScore {
    pub fn scores(&self) -> (Score, Score) {
        (self.get(0).pending, self.get(1).pending)
    }
}

impl Default for AccountScore {
    fn default() -> Self {
        Self {
            updated_at: block_timestamp_ms().into(),
            history: [DailyScore::default(); DAYS_STORED],
        }
    }
}
