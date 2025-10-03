use std::u16;

use near_sdk::{
    env::{block_timestamp_ms, panic_str},
    near,
};

use crate::{Day, DaysOffset, Local, TimeHelper, Timestamp, Timezone, UDecimal, UTC};

mod common;

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

pub type ScoreIncrement = (Score, UTC);
pub type ScoreIncrements = Vec<ScoreIncrement>;

#[near(serializers=[borsh, json])]
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct DailyScore {
    pub value: Score,
    pub is_settled: bool,
}

#[near(serializers=[borsh, json])]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AccountScore {
    updated_at: UTC,
    #[deprecated]
    pub timezone: Timezone,
    /// Daily score history with settlement status
    history: [DailyScore; DAYS_STORED],
}

impl AccountScore {
    pub fn updated_at(&self) -> Timestamp {
        self.updated_at.0
    }

    fn get(&self, days_ago: DaysOffset) -> DailyScore {
        self.assert_in_bounds(days_ago as usize);
        self.history[days_ago as usize]
    }

    fn get_mut(&mut self, days_ago: DaysOffset) -> &mut DailyScore {
        self.assert_in_bounds(days_ago as usize);
        &mut self.history[days_ago as usize]
    }

    fn set(&mut self, days_ago: DaysOffset, score: DailyScore) {
        self.assert_in_bounds(days_ago as usize);
        self.history[days_ago as usize] = score;
    }

    fn add(&mut self, days_ago: DaysOffset, increment: Score) {
        let current_value = self.get(days_ago).value;
        self.get_mut(days_ago).value = current_value.checked_add(increment).unwrap_or(u16::MAX);
    }

    fn wipe(&mut self) {
        self.history = [DailyScore::default(); DAYS_STORED];
    }

    fn shift(&mut self) {
        self.history.copy_within(0..DAYS_STORED - 1, 1);
    }

    fn settle_at(&mut self, days_ago: DaysOffset) -> Score {
        self.get_mut(days_ago).is_settled = true;

        self.get(days_ago).value
    }

    fn assert_in_bounds(&self, index: usize) {
        if index >= DAYS_STORED {
            panic_str(format!("{index} is out of range. Only store {DAYS_STORED} days.").as_str());
        }
    }

    pub fn get_pending_scores(&self, timezone: Timezone) -> ScoreRecord {
        ScoreRecord {
            score: self
                .get_finalized_scores(timezone)
                .iter()
                .filter_map(|item| if item.is_settled { None } else { Some(item.value) })
                .collect(),
            updated: self.updated_at,
        }
    }

    pub fn get_last_finalized_score(&self, timezone: Timezone) -> Score {
        match self.get_days_number_since_last_update(timezone) {
            // Updated today => 0 offsetted day's score is still ongoing. Return last finalized value.
            0 => self.get(1).value,
            // Updated earlier than today => 0 offsetted day is finalized.
            1 => self.get(0).value,
            _ => 0,
        }
    }

    pub fn settle(&mut self, timezone: Timezone) -> Vec<Score> {
        let days_since_last_update = self.get_days_number_since_last_update(timezone);

        let settled_scores = if days_since_last_update == 0 {
            vec![self.settle_at(1)]
        } else {
            vec![self.settle_at(0), self.settle_at(1)]
        };

        if days_since_last_update == 1 {
            self.shift();
        } else {
            self.wipe();
        }

        settled_scores
    }

    pub fn update(&mut self, timezone: Timezone, increments: ScoreIncrements) {
        for (increment, days_ago) in increments {
            self.add(days_ago, increment);
        }

        self.updated_at = block_timestamp_ms().into();
    }

    // If the score's last update day is yesterday or earlier relative to the today parameter, all historical score records are deemed finalized.
    // Conversely, if the function is called on the same day as the score's last update, the score at index 0 (representing the current day)
    // is considered active/ongoing and is excluded from the calculation.
    fn get_finalized_scores(&self, timezone: Timezone) -> Vec<DailyScore> {
        if timezone.today() > self.get_update_day(timezone) {
            self.history.into()
        } else {
            self.history[1..].into()
        }
    }

    pub fn get_update_day(&self, timezone: Timezone) -> Day {
        timezone.adjust(self.updated_at).day()
    }

    pub fn get_days_number_since_last_update(&self, timezone: Timezone) -> DaysOffset {
        (timezone.today().0 - self.get_update_day(timezone).0) as DaysOffset
    }
}

pub trait ScoreFilter {
    fn filter(&self, timezone: Timezone) -> (Vec<ScoreIncrement>, Vec<ScoreIncrement>);
}

impl ScoreFilter for ScoreIncrements {
    fn filter(&self, timezone: Timezone) -> (Vec<ScoreIncrement>, Vec<ScoreIncrement>) {
        let mut valid_increments = vec![];
        let mut outdated_increments = vec![];
        for increment in self {
            timezone.assert_not_future(increment.1);

            let days_ago = timezone.today() - timezone.adjust(increment.1).day();
            if days_ago >= DAYS_STORED.into() {
                outdated_increments.push(*increment);
            } else {
                valid_increments.push(*increment);
            }
        }

        (outdated_increments, valid_increments)
    }
}
