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

impl ToAPY for u32 {
    /// 1000 scores = 1%
    fn to_apy(self) -> UDecimal {
        UDecimal::new(self.into(), 5)
    }
}

pub type ScoreIncrement = (Score, UTC);
pub type ScoreIncrements = Vec<ScoreIncrement>;

#[near(serializers=[borsh, json])]
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct DailyScore {
    pub value: Score,
    pub booster: BoostedScore,
}

#[near(serializers=[json])]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct DailyScoreView {
    pub value: Score,
    pub booster: Score,
}

impl From<DailyScore> for DailyScoreView {
    fn from(value: DailyScore) -> Self {
        Self {
            value: value.value,
            booster: value.booster.get_value(),
        }
    }
}

impl DailyScore {
    pub fn new(value: Score) -> Self {
        Self {
            value,
            booster: BoostedScore::default(),
        }
    }

    pub fn to_capped_apy(&self, cap: Score, include_booster: bool) -> UDecimal {
        dbg!(self);
        (self.value.min(cap) + if include_booster { self.booster.get_value() } else { 0 }).to_apy()
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
    pub history: [DailyScore; DAYS_STORED],
}

#[near(serializers=[json])]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct AccountScoreView {
    pub updated_at: UTC,
    pub history: Vec<DailyScoreView>,
}

impl From<AccountScore> for AccountScoreView {
    fn from(value: AccountScore) -> Self {
        Self {
            updated_at: value.updated_at,
            history: value.history.iter().copied().map(DailyScoreView::from).collect(),
        }
    }
}

impl AccountScore {
    pub fn new(updated_at: UTC, history: [DailyScore; DAYS_STORED]) -> Self {
        Self { updated_at, history }
    }

    pub fn updated_at(&self) -> Timestamp {
        self.updated_at.0
    }

    pub fn get(&self, days_ago: DaysOffset) -> DailyScore {
        self.assert_in_bounds(days_ago as usize);
        self.history[days_ago as usize]
    }

    fn get_mut(&mut self, days_ago: DaysOffset) -> &mut DailyScore {
        self.assert_in_bounds(days_ago as usize);
        &mut self.history[days_ago as usize]
    }

    #[allow(dead_code)]
    fn set(&mut self, days_ago: DaysOffset, score: DailyScore) {
        self.assert_in_bounds(days_ago as usize);
        self.history[days_ago as usize] = score;
    }

    fn add(&mut self, days_ago: DaysOffset, increment: Score) {
        let score = self.get_mut(days_ago);

        score.value = score.value.saturating_add(increment);
    }

    pub fn apply_booster(&mut self, days_ago: DaysOffset, value: Score) -> bool {
        if self.get(days_ago).booster.get_value() > 0 {
            return false;
        }

        self.get_mut(days_ago).booster = BoostedScore::new(value, false);

        true
    }

    pub fn wipe(&mut self) {
        self.history = [DailyScore::default(); DAYS_STORED];
    }

    pub fn shift(&mut self) {
        self.history.copy_within(0..DAYS_STORED - 1, 1);
        self.history[0] = DailyScore::default();
    }

    #[allow(clippy::unused_self)]
    fn assert_in_bounds(&self, index: usize) {
        if index >= DAYS_STORED {
            panic_str(format!("{index} is out of range. Only store {DAYS_STORED} days.").as_str());
        }
    }

    pub fn get_capped_finalized_score(&self, timezone: Timezone, total_cap: Score) -> Score {
        let days_since_last_update = self.get_days_number_since_last_update(timezone);

        if days_since_last_update > 1 {
            return 0;
        }

        self.get(1).value.min(total_cap)
    }

    pub fn get_last_finalized_record(&self, timezone: Timezone) -> DailyScore {
        match self.get_days_number_since_last_update(timezone) {
            // Updated today => 0 offsetted day's score is still ongoing. Return last finalized value.
            0 => self.get(1),
            // Updated earlier than today => 0 offsetted day is finalized.
            1 => self.get(0),
            _ => DailyScore::default(),
        }
    }

    pub fn update(&mut self, increments: Vec<(Score, DaysOffset)>) {
        for (increment, days_ago) in increments {
            self.add(days_ago, increment);
        }

        self.updated_at = block_timestamp_ms().into();
    }

    pub fn get_update_day(&self, timezone: Timezone) -> Day {
        timezone.adjust(self.updated_at).day()
    }

    pub fn get_days_number_since_last_update(&self, timezone: Timezone) -> DaysOffset {
        DaysOffset::try_from(timezone.today().0 - self.get_update_day(timezone).0)
            .unwrap_or_else(|_| panic_str("Failed to calculate days offset"))
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
        .map(|increment| {
            let days_offset = DaysOffset::try_from(timezone.today().0 - increment.1.day().0)
                .unwrap_or_else(|_| panic_str("Failed to calculate days offset"));
            (increment.0, days_offset)
        })
        .collect()
}

impl From<AccountScoreLegacy> for AccountScore {
    fn from(value: AccountScoreLegacy) -> Self {
        let mut history = [DailyScore::default(); DAYS_STORED];
        for (i, item) in history.iter_mut().enumerate().take(DAYS_STORED) {
            item.value = value.scores_history[i];
        }

        Self {
            updated_at: value.updated_at,
            history,
        }
    }
}
