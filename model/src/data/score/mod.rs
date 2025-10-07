use std::u16;

use near_sdk::{
    env::{block_timestamp_ms, panic_str},
    near,
};

use crate::{Day, DaysOffset, Local, TimeHelper, Timestamp, Timezone, UDecimal, UTC};

mod booster;
mod common;

pub use booster::BoostedScore;

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
    pub pending: Score,
    pub total: Score,
}

impl DailyScore {
    pub fn new(value: Score) -> Self {
        Self {
            pending: value,
            total: value,
        }
    }
}

#[near(serializers=[borsh, json])]
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct AccountScoreLegacy {
    pub updated_at: UTC,
    pub timezone: Timezone,
    pub scores: [Score; DAYS_STORED],
    pub scores_history: [Score; DAYS_STORED],
}

#[near(serializers=[borsh, json])]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AccountScore {
    updated_at: UTC,
    history: [DailyScore; DAYS_STORED],
}

impl AccountScore {
    pub fn new(updated_at: UTC, history: [DailyScore; DAYS_STORED]) -> Self {
        Self { updated_at, history }
    }

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
        let score = self.get_mut(days_ago);

        score.pending = score.pending.checked_add(increment).unwrap_or(u16::MAX);
        score.total = score.total.checked_add(increment).unwrap_or(u16::MAX);
    }

    fn wipe(&mut self) {
        self.history = [DailyScore::default(); DAYS_STORED];
    }

    fn shift(&mut self) {
        self.history.copy_within(0..DAYS_STORED - 1, 1);
        self.history[0] = DailyScore::default();
    }

    fn settle_at(&mut self, days_ago: DaysOffset) -> Score {
        let result = self.get(days_ago).pending;
        self.get_mut(days_ago).pending = 0;

        result
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
                .filter_map(|item| if item.pending == 0 { None } else { Some(item.pending) })
                .collect(),
            updated: self.updated_at,
        }
    }

    pub fn get_last_finalized_score(&self, timezone: Timezone) -> Score {
        match self.get_days_number_since_last_update(timezone) {
            // Updated today => 0 offsetted day's score is still ongoing. Return last finalized value.
            0 => self.get(1).total,
            // Updated earlier than today => 0 offsetted day is finalized.
            1 => self.get(0).total,
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
        } else if days_since_last_update > 1 {
            self.wipe();
        }

        self.updated_at = block_timestamp_ms().into();

        settled_scores
    }

    pub fn update(&mut self, increments: Vec<(Score, DaysOffset)>) {
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

pub struct ScoreIncrementProcessor<'a> {
    scores: &'a Vec<(Score, UTC)>,
    timezone: Timezone,
}

impl<'a> ScoreIncrementProcessor<'a> {
    pub fn new(scores: &'a Vec<(Score, UTC)>, timezone: Timezone) -> Self {
        Self { scores, timezone }
    }

    pub fn process(&self) -> SegmentedScoreIncrements {
        let mut result = SegmentedScoreIncrements::default();

        for increment in self.scores {
            self.verify_timestamp(increment);

            let adjusted_increment = self.adjust_timestamp(increment);
            if self.is_valid(&adjusted_increment) {
                result.valid.push(adjusted_increment);
            } else {
                result.outdated.push(adjusted_increment);
            }
        }

        result
    }

    fn verify_timestamp(&self, increment: &(Score, UTC)) {
        self.timezone.assert_not_future(increment.1);
    }

    fn adjust_timestamp(&self, increment: &(Score, UTC)) -> (Score, Local) {
        (increment.0, self.timezone.adjust(increment.1))
    }

    fn is_valid(&self, increment: &(Score, Local)) -> bool {
        self.timezone.today().0 - increment.1.day().0 < DAYS_STORED as _
    }
}

#[derive(Default)]
pub struct SegmentedScoreIncrements {
    pub outdated: Vec<(Score, Local)>,
    pub valid: Vec<(Score, Local)>,
}

pub fn convert_to_days_offset(input: Vec<(Score, Local)>, timezone: Timezone) -> Vec<(Score, DaysOffset)> {
    input
        .iter()
        .map(|increment| (increment.0, (timezone.today().0 - increment.1.day().0) as _))
        .collect()
}

impl From<AccountScoreLegacy> for AccountScore {
    fn from(value: AccountScoreLegacy) -> Self {
        let mut history = [DailyScore::default(); DAYS_STORED];
        for i in 0..DAYS_STORED {
            history[i].pending = value.scores[i];
            history[i].total = value.scores_history[i];
        }

        Self {
            updated_at: value.updated_at,
            history,
        }
    }
}
