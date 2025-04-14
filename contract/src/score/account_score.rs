use near_sdk::env::{block_timestamp_ms, panic_str};
use sweat_jar_model::{AccountScore, Chain, Day, TimeHelper, DAYS_STORED};

use crate::event::{emit, EventKind};

pub trait AccountScoreUpdate {
    fn update(&mut self, chain: Chain);
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
mod test {
    use near_sdk::env::block_timestamp_ms;
    use sweat_jar_model::{
        data::{
            account::Account,
            product::{test_utils::DEFAULT_SCORE_PRODUCT_NAME, Cap, Product, ScoreBasedProductTerms, Terms},
        },
        interest::InterestCalculator,
        AccountScore, Day, Timezone, MS_IN_DAY, MS_IN_HOUR, MS_IN_YEAR, UTC,
    };

    use crate::{common::tests::Context, score::account_score::Chain, test_utils::admin};

    use super::AccountScoreUpdate;

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
        let mut ctx = Context::new(admin());
        let product = generate_score_based_product();

        let mut now = TODAY;
        ctx.set_block_timestamp_in_ms(now);

        let mut score = AccountScore::new(TIMEZONE);
        score.update(generate_chain());
        let mut account = Account {
            score,
            ..Account::default()
        };

        assert_eq!(0.03, product.terms.get_apy(&account).to_f32());

        now += MS_IN_DAY;
        ctx.set_block_timestamp_in_ms(now);
        assert_eq!(0.05, product.terms.get_apy(&account).to_f32());

        now += MS_IN_DAY;
        ctx.set_block_timestamp_in_ms(now);
        assert_eq!(0.05, product.terms.get_apy(&account).to_f32());

        assert_eq!(vec![2000, 3000], account.score.reset_score().score);
        assert_eq!(0.00, product.terms.get_apy(&account).to_f32());
    }

    #[test]
    #[should_panic(expected = "Walk data from future")]
    fn steps_from_future() {
        let mut ctx = Context::new(admin());
        ctx.set_block_timestamp_in_ms(TODAY);

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

        let mut ctx = Context::new(admin());

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 10);

        score.update(vec![(6, (MS_IN_DAY * 10).into()), (5, (MS_IN_DAY * 9).into())]);

        assert_eq!(score.updated, (MS_IN_DAY * 10).into());
        assert_eq!(score.scores(), (1006, 2005));
        assert_eq!(score.reset_score().score, vec![2005]);
        assert_eq!(score.active_score(), 2005);

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 11);
        assert_eq!(score.reset_score().score, vec![1006, 0]);
        assert_eq!(score.active_score(), 1006);

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 12);
        assert_eq!(score.reset_score().score, vec![0, 0]);
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

        let mut ctx = Context::new(admin());

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 10);

        assert_eq!(score.active_score(), 2000);

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 11);

        assert_eq!(score.active_score(), 1000);

        ctx.set_block_timestamp_in_ms(MS_IN_DAY * 12);

        assert_eq!(score.active_score(), 0);
    }

    fn generate_score_based_product() -> Product {
        Product {
            id: DEFAULT_SCORE_PRODUCT_NAME.to_string(),
            cap: Cap::new(0, 100_000_000 * 10u128.pow(18)),
            terms: Terms::ScoreBased(ScoreBasedProductTerms {
                score_cap: 20_000,
                lockup_term: MS_IN_YEAR.into(),
            }),
            withdrawal_fee: None,
            public_key: None,
            is_enabled: true,
        }
    }
}
