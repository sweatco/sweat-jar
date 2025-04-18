use near_sdk::env::block_timestamp_ms;

use super::DAYS_STORED;
use crate::{AccountScore, Chain, Day, Local, Score, ScoreRecord, TimeHelper, Timezone};

impl AccountScore {
    pub fn is_valid(&self) -> bool {
        self.timezone.is_valid()
    }

    pub fn new(timezone: Timezone) -> Self {
        Self {
            updated: block_timestamp_ms().into(),
            timezone,
            scores: [0; DAYS_STORED],
            scores_history: [0; DAYS_STORED],
        }
    }

    pub fn scores(&self) -> (Score, Score) {
        (self.scores[0], self.scores[1])
    }

    pub fn claimable_score(&self) -> ScoreRecord {
        let score = if self.update_day() == self.timezone.today() {
            vec![self.scores[1]]
        } else {
            vec![self.scores[0], self.scores[1]]
        };

        ScoreRecord {
            score,
            updated: self.updated,
        }
    }

    pub fn active_score(&self) -> Score {
        let update_day = self.update_day();
        let today = self.timezone.today();

        if update_day == today {
            self.scores_history[1]
        } else if update_day == Local(today.0 - 1) {
            self.scores_history[0]
        } else {
            0
        }
    }

    pub fn try_reset_score(&mut self) {
        if self.is_valid() {
            self.reset_score();
        }
    }

    /// On claim we need to clear active scores so they aren't claimed twice or more.
    pub fn reset_score(&mut self) -> ScoreRecord {
        let today = self.timezone.today();
        let update_day = self.update_day();

        let score = if today == update_day {
            let score = self.scores[1];
            self.scores[1] = 0;
            vec![score]
        } else {
            let score = vec![self.scores[0], self.scores[1]];
            self.scores[0] = 0;
            self.scores[1] = 0;

            // If scores were updated yesterday we shift history by 1 day
            // If older that yesterday then we wipe it
            if update_day == Local(today.0 - 1) {
                self.scores_history[1] = self.scores_history[0];
                self.scores_history[0] = 0;
            } else {
                self.scores_history = [0; DAYS_STORED];
            }

            score
        };

        let updated = self.updated;

        self.updated = block_timestamp_ms().into();

        ScoreRecord { score, updated }
    }

    /// Update on the same day - just add values
    pub fn update_today(&mut self, chain: Chain) -> Vec<Score> {
        for (score, day) in chain {
            let day_index: usize = day.0.try_into().unwrap();
            self.scores[day_index] = self.scores[day_index].checked_add(score).unwrap_or(u16::MAX);
            self.scores_history[day_index] = self.scores_history[day_index].checked_add(score).unwrap_or(u16::MAX);
        }
        vec![]
    }

    pub fn update_day(&self) -> Day {
        self.timezone.adjust(self.updated).day()
    }
}

impl Default for AccountScore {
    fn default() -> Self {
        Self {
            updated: block_timestamp_ms().into(),
            timezone: Timezone::invalid(),
            scores: [0, 0],
            scores_history: [0, 0],
        }
    }
}
