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
        fn with_pending_withdraw(self) -> Self;
    }

    impl JarBuilder for Jar {
        fn with_pending_withdraw(mut self) -> Self {
            self.is_pending_withdraw = true;
            self
        }
    }
}
