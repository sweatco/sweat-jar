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
    pub pending: Score,
    pub total: Score,
    pub booster: BoostedScore,
}

#[near(serializers=[json])]
#[derive(Default, Debug, PartialEq, Clone)]
pub struct DailyScoreView {
    pub pending: Score,
    pub total: Score,
    pub booster: Score,
    pub is_booster_claimed: bool,
}

impl From<DailyScore> for DailyScoreView {
    fn from(value: DailyScore) -> Self {
        Self {
            pending: value.pending,
            total: value.total,
            booster: value.booster.get_value(),
            is_booster_claimed: value.booster.is_claimed(),
        }
    }
}

impl DailyScore {
    pub fn new(value: Score) -> Self {
        Self {
            pending: value,
            total: value,
            booster: BoostedScore::default(),
        }
    }

    pub fn settled_score(&self) -> Score {
        self.total - self.pending
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

    fn get(&self, days_ago: DaysOffset) -> DailyScore {
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

        score.pending = score.pending.saturating_add(increment);
        score.total = score.total.saturating_add(increment);
    }

    pub fn apply_booster(&mut self, days_ago: DaysOffset, value: Score) -> bool {
        if self.get(days_ago).booster.get_value() > 0 {
            return false;
        }

        self.get_mut(days_ago).booster = BoostedScore::new(value, false);

        true
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
        self.get_mut(days_ago).booster.set_claimed(true);

        result
    }

    #[allow(clippy::unused_self)]
    fn assert_in_bounds(&self, index: usize) {
        if index >= DAYS_STORED {
            panic_str(format!("{index} is out of range. Only store {DAYS_STORED} days.").as_str());
        }
    }

    pub fn get_capped_pending_score(&self, timezone: Timezone, total_cap: Score) -> Score {
        self.get_finalized_scores(timezone)
            .iter()
            .map(|item| {
                if item.total <= total_cap {
                    item.pending
                } else {
                    let settled = item.total - item.pending;
                    total_cap.saturating_sub(settled)
                }
            })
            .sum()
    }

    pub fn get_capped_total_finalized_score(&self, timezone: Timezone, total_cap: Score) -> Score {
        self.get_finalized_scores(timezone)
            .iter()
            .map(|item| item.total.min(total_cap))
            .sum()
    }

    pub fn get_pending_finalized_boosters(&self, timezone: Timezone) -> Score {
        self.get_finalized_scores(timezone)
            .iter()
            .map(|item| {
                if item.booster.is_claimed() {
                    0
                } else {
                    item.booster.get_value()
                }
            })
            .sum()
    }

    pub fn get_finalized_boosters(&self, timezone: Timezone) -> Score {
        self.get_finalized_scores(timezone)
            .iter()
            .map(|item| item.booster.get_value())
            .sum()
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

        match days_since_last_update.cmp(&1) {
            std::cmp::Ordering::Equal => self.shift(),
            std::cmp::Ordering::Greater => self.wipe(),
            std::cmp::Ordering::Less => {}
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
            item.pending = value.scores[i];
            item.total = value.scores_history[i];
        }

        Self {
            updated_at: value.updated_at,
            history,
        }
    }
}
