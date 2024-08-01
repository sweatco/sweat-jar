use std::convert::TryInto;

use near_sdk::{
    env::{block_timestamp_ms, panic_str},
    log, near,
};
use sweat_jar_model::{Day, Local, Score, TimeHelper, Timezone, UTC};

const DAYS_STORED: usize = 2;

type Chain = Vec<(Score, Local)>;

#[near]
#[derive(Copy, Clone, Debug)]
pub struct AccountScore {
    pub updated: UTC,
    pub timezone: Timezone,
    scores: [Score; DAYS_STORED],
}

impl AccountScore {
    pub fn new(timezone: Timezone) -> Self {
        Self {
            updated: block_timestamp_ms().into(),
            timezone,
            scores: [0; DAYS_STORED],
        }
    }

    pub fn claimable_score(&self) -> Vec<Score> {
        if self.update_day() == self.timezone.today() {
            vec![self.scores[1]]
        } else {
            vec![self.scores[0], self.scores[1]]
        }
    }

    /// On claim we need to clear active scores so they aren't claimed twice or more.
    pub fn claim_score(&mut self) -> Vec<Score> {
        let today = self.timezone.today();

        let result = if today == self.update_day() {
            let score = self.scores[1];
            self.scores[1] = 0;
            vec![score]
        } else {
            let score = vec![self.scores[0], self.scores[1]];
            self.scores[0] = 0;
            self.scores[1] = 0;
            score
        };

        self.updated = block_timestamp_ms().into();

        result
    }

    pub fn update(&mut self, chain: Chain) -> Vec<Score> {
        let today = self.timezone.today();

        let chain = self.convert_chain(today, chain);

        let result = match (today - self.update_day()).0 {
            0 => self.update_today(chain),
            1 => self.update_yesterday(chain),
            _ => self.update_older_than_yesterday(chain),
        };

        self.updated = block_timestamp_ms().into();

        result
    }

    /// Update on the same day - just add values
    fn update_today(&mut self, chain: Chain) -> Vec<Score> {
        for (score, day) in chain {
            let day_index: usize = day.0.try_into().unwrap();
            self.scores[day_index] += score;
        }
        vec![]
    }

    /// Last update was yesterday. We need to shift values by 1 day and return score for last day.
    fn update_yesterday(&mut self, chain: Chain) -> Vec<Score> {
        let score = self.scores[1];

        self.scores[1] = self.scores[0];
        self.scores[0] = 0;

        self.update_today(chain);

        vec![score]
    }

    /// Last update was 2 or more days ago. Reset and return steps for both days.
    fn update_older_than_yesterday(&mut self, chain: Chain) -> Vec<Score> {
        let score = vec![self.scores[0], self.scores[1]];

        self.scores[0] = 0;
        self.scores[1] = 0;

        self.update_today(chain);

        score
    }

    fn update_day(&self) -> Day {
        self.timezone.adjust(self.updated).day()
    }

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

                let day = today - timestamp.day();

                if day >= DAYS_STORED.into() {
                    log!(
                        "WARN: Walk data is too old: {:?}. It will be ignored",
                        (score, timestamp)
                    );
                    return None;
                }

                (score, day).into()
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use std::time::{SystemTime, UNIX_EPOCH};

    use near_sdk::env::block_timestamp_ms;
    use sweat_jar_model::{Day, Timezone, MS_IN_DAY, MS_IN_HOUR};

    use crate::{product::model::Product, score::account_score::Chain, test_builder::TestBuilder, AccountScore};

    const TIMEZONE: Timezone = Timezone::hour_shift(3);

    fn generate_chain() -> Chain {
        let start = SystemTime::now();
        let today: u64 = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            .try_into()
            .unwrap();

        let today: Day = today.into();

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

        ctx.set_block_timestamp_today();

        let product = Product::new().score_cap(20_000);

        let mut account_score = AccountScore::new(TIMEZONE);

        account_score.update(generate_chain());

        assert_eq!(product.apy_for_score(&account_score.claimable_score()).to_f32(), 0.03);

        ctx.advance_block_timestamp_days(1);
        assert_eq!(product.apy_for_score(&account_score.claimable_score()).to_f32(), 0.05);

        ctx.advance_block_timestamp_days(1);
        assert_eq!(product.apy_for_score(&account_score.claimable_score()).to_f32(), 0.05);

        assert_eq!(account_score.claim_score(), vec![2000, 3000]);

        assert_eq!(product.apy_for_score(&account_score.claimable_score()).to_f32(), 0.00);
    }

    #[test]
    #[should_panic(expected = "Walk data from future")]
    fn steps_from_future() {
        let mut ctx = TestBuilder::new().build();
        ctx.set_block_timestamp_today();

        let mut account_score = AccountScore::new(TIMEZONE);
        account_score.update(vec![(1_000, (block_timestamp_ms() + MS_IN_DAY).into())]);
    }
}
