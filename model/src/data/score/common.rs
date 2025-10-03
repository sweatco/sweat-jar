use near_sdk::env::block_timestamp_ms;

use super::{DailyScore, DAYS_STORED};
use crate::{AccountScore, Day, Local, Score, ScoreRecord, TimeHelper, Timezone};

impl AccountScore {
    pub fn scores(&self) -> (Score, Score) {
        (self.get(0).value, self.get(1).value)
    }
}

impl Default for AccountScore {
    fn default() -> Self {
        Self {
            updated_at: block_timestamp_ms().into(),
            timezone: Timezone::invalid(),
            history: [DailyScore::default(); DAYS_STORED],
        }
    }
}
