use sweat_jar_model::{convert_to_days_offset, data::account::Account, ScoreIncrementProcessor, ScoreIncrements};

use crate::common::event::{emit, EventKind};

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
