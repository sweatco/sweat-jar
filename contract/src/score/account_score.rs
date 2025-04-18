use std::convert::TryInto;

use near_sdk::{
    env::{block_timestamp_ms, panic_str},
    near,
};
use sweat_jar_model::{Day, Local, Score, ScoreRecord, TimeHelper, Timezone, UTC};

use crate::event::{emit, EventKind};

const DAYS_STORED: usize = 2;

type Chain = Vec<(Score, Local)>;

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

    /// On claim we need to clear active scores so they aren't claimed twice or more.
    pub fn claim_score(&mut self) -> ScoreRecord {
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

    pub fn update(&mut self, chain: Chain) {
        let today = self.timezone.today();

        let chain = self.convert_chain(today, chain);

        assert_eq!((today - self.update_day()).0, 0, "Updating scores before claiming them");

        self.update_today(chain);

        self.updated = block_timestamp_ms().into();
    }

    /// Update on the same day - just add values
    fn update_today(&mut self, chain: Chain) -> Vec<Score> {
        for (score, day) in chain {
            let day_index: usize = day.0.try_into().unwrap();
            self.scores[day_index] = self.scores[day_index].checked_add(score).unwrap_or(u16::MAX);
            self.scores_history[day_index] = self.scores_history[day_index].checked_add(score).unwrap_or(u16::MAX);
        }
        vec![]
    }

    fn update_day(&self) -> Day {
        self.timezone.adjust(self.updated).day()
    }

    /// Convert walkchain timestamps to days
    fn convert_chain(&self, today: Day, walkchain: Chain) -> Chain {
        let now = self.timezone.now();
        walkchain
            .into_iter()
            .filter_map(|(score, timestamp)| {
                if timestamp > now {
                    panic_str(&format!(
                        "Walk data from future: {:?}. Now: {:?}",
                        (score, timestamp),
                        now
                    ));
                }

                let days_ago = today - timestamp.day();

                if days_ago >= DAYS_STORED.into() {
                    emit(EventKind::OldScoreWarning((score, timestamp)));

                    return None;
                }

                (score, days_ago).into()
            })
            .collect()
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

#[cfg(test)]
mod test {
    use near_sdk::env::block_timestamp_ms;
    use sweat_jar_model::{Day, Timezone, MS_IN_DAY, MS_IN_HOUR, UTC};

    use crate::{
        product::model::Product,
        score::{account_score::Chain, AccountScore},
        test_builder::TestBuilder,
    };

    const TIMEZONE: Timezone = Timezone::hour_shift(3);
    const TODAY: u64 = 1722234632000;

    fn generate_chain() -> Chain {
        let today: Day = TODAY.into();

        vec![
            (1_000, today),
            (1_000, today - (MS_IN_HOUR * 3).into()),
            (1_000, today - (MS_IN_HOUR * 12).into()),
            (1_000, today - (MS_IN_HOUR * 25).into()),
            (1_000, today - (MS_IN_HOUR * 28).into()),
            (1_000, today - (MS_IN_HOUR * 40).into()),
            (1_000, today - (MS_IN_HOUR * 45).into()),
            (1_000, today - (MS_IN_HOUR * 48).into()),
            (1_000, today - (MS_IN_HOUR * 55).into()),
            (1_000, today - (MS_IN_HOUR * 550).into()),
        ]
    }

    #[test]
    fn test_account_score() {
        let mut ctx = TestBuilder::new().build();

        ctx.set_block_timestamp_in_ms(TODAY);

        let product = Product::new().score_cap(20_000);

        let mut account_score = AccountScore::new(TIMEZONE);

        account_score.update(generate_chain());

        assert_eq!(
            product.apy_for_score(&account_score.claimable_score().score).to_f32(),
            0.03
        );

        ctx.advance_block_timestamp_days(1);
        assert_eq!(
            product.apy_for_score(&account_score.claimable_score().score).to_f32(),
            0.05
        );

        ctx.advance_block_timestamp_days(1);
        assert_eq!(
            product.apy_for_score(&account_score.claimable_score().score).to_f32(),
            0.05
        );

        assert_eq!(account_score.claim_score().score, vec![2000, 3000]);

        assert_eq!(
            product.apy_for_score(&account_score.claimable_score().score).to_f32(),
            0.00
        );
    }

    #[test]
    #[should_panic(expected = "Walk data from future")]
    fn steps_from_future() {
        let mut ctx = TestBuilder::new().build();
        ctx.set_block_timestamp_today();

        let mut account_score = AccountScore::new(TIMEZONE);
        account_score.update(vec![(1_000, (block_timestamp_ms() + MS_IN_DAY).into())]);
    }

    #[test]
    fn updated_on_different_days() {
        let mut score = AccountScore {
            updated: UTC(MS_IN_DAY * 10),
            timezone: Timezone::hour_shift(0),
            scores: [1000, 2000],
            scores_history: [1000, 2000],
        };

        let mut ctx = TestBuilder::new().build();

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 10);

        score.update(vec![(6, (MS_IN_DAY * 10).into()), (5, (MS_IN_DAY * 9).into())]);

        assert_eq!(score.updated, (MS_IN_DAY * 10).into());
        assert_eq!(score.scores(), (1006, 2005));
        assert_eq!(score.claim_score().score, vec![2005]);
        assert_eq!(score.active_score(), 2005);

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 11);
        assert_eq!(score.claim_score().score, vec![1006, 0]);
        assert_eq!(score.active_score(), 1006);

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 12);
        assert_eq!(score.claim_score().score, vec![0, 0]);
        assert_eq!(score.active_score(), 0);
    }

    #[test]
    fn active_score() {
        let score = AccountScore {
            updated: UTC(MS_IN_DAY * 10),
            timezone: Timezone::hour_shift(0),
            scores: [1000, 2000],
            scores_history: [1000, 2000],
        };

        let mut ctx = TestBuilder::new().build();

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 10);

        assert_eq!(score.active_score(), 2000);

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 11);

        assert_eq!(score.active_score(), 1000);

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 12);

        assert_eq!(score.active_score(), 0);
    }
}
