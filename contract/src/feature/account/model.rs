use near_sdk::env::{block_timestamp_ms, panic_str};
use sweat_jar_model::{AccountScore, Chain, Day, Score, TimeHelper, Timezone, DAYS_STORED, UTC};

use crate::common::event::{emit, EventKind};

pub trait AccountScoreUpdate {
    fn update(&mut self, chain: Chain);
}

pub trait ScoreConverter {
    /// Convert Score to a User's timezone
    fn adjust(&self, timezone: Timezone) -> Chain;
}

impl ScoreConverter for Vec<(Score, UTC)> {
    fn adjust(&self, timezone: Timezone) -> Chain {
        self.iter().map(|score| (score.0, timezone.adjust(score.1))).collect()
    }
}

impl AccountScoreUpdate for AccountScore {
    fn update(&mut self, chain: Chain) {
        let today = self.timezone.today();

        let chain = convert_chain(self, today, chain);

        assert_eq!((today - self.update_day()).0, 0, "Updating scores before claiming them");

        self.update_today(chain);
        self.updated = block_timestamp_ms().into();
    }
}

/// Convert walkchain timestamps to days
fn convert_chain(score: &AccountScore, today: Day, walkchain: Chain) -> Chain {
    let now = score.timezone.now();
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

#[cfg(test)]
pub(crate) mod test_utils {
    use rstest::fixture;
    use sweat_jar_model::{
        data::jar::{Deposit, Jar},
        Timestamp, TokenAmount,
    };

    #[fixture]
    pub fn jar(#[default(vec![])] deposits: Vec<(Timestamp, TokenAmount)>) -> Jar {
        Jar {
            deposits: deposits
                .into_iter()
                .map(|(created_at, principal)| Deposit::new(created_at, principal))
                .collect(),
            cache: None,
            is_pending_withdraw: false,
            claim_remainder: 0,
        }
    }

    pub(crate) trait JarBuilder {
        fn with_deposit(self, created_at: Timestamp, principal: TokenAmount) -> Self;
        fn with_deposits(self, deposits: Vec<(Timestamp, TokenAmount)>) -> Self;
        fn with_pending_withdraw(self) -> Self;
    }

    impl JarBuilder for Jar {
        fn with_deposit(mut self, created_at: Timestamp, principal: TokenAmount) -> Self {
            self.deposits.push(Deposit::new(created_at, principal));
            self
        }

        fn with_deposits(mut self, deposits: Vec<(Timestamp, TokenAmount)>) -> Self {
            self.deposits.extend(
                deposits
                    .into_iter()
                    .map(|(created_at, deposit)| Deposit::new(created_at, deposit)),
            );
            self
        }

        fn with_pending_withdraw(mut self) -> Self {
            self.is_pending_withdraw = true;
            self
        }
    }
}
