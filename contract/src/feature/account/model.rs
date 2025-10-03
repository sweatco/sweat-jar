use sweat_jar_model::{
    data::account::Account, AccountScore, Day, Score, ScoreFilter, ScoreIncrements, TimeHelper, Timezone, DAYS_STORED,
    UTC,
};

use crate::common::event::{emit, EventKind};

pub trait AccountScoreUpdate {
    fn update(&mut self, increments: ScoreIncrements);
}

pub trait ScoreConverter {
    /// Convert Score to a User's timezone
    fn adjust(&self, timezone: Timezone) -> ScoreIncrements;
}

impl ScoreConverter for Vec<(Score, UTC)> {
    fn adjust(&self, timezone: Timezone) -> ScoreIncrements {
        self.iter().map(|score| (score.0, timezone.adjust(score.1))).collect()
    }
}

impl AccountScoreUpdate for Account {
    fn update(&mut self, increments: ScoreIncrements) {
        assert_eq!(
            self.score.get_days_number_since_last_update(self.timezone),
            0,
            "Updating scores before settlement"
        );

        todo!("Introdure ScoreIncrement with local and utc timestamps. Filter function should return local values.");
        let (outdated_increments, valid_increments) = increments.filter(self.timezone);

        for (score, timestamp) in outdated_increments {
            emit(EventKind::OldScoreWarning((score, timestamp)));
        }

        self.score.update(self.timezone, valid_increments);
    }
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
