#![cfg(test)]

use fake::Fake;
use near_sdk::{
    json_types::{I64, U128},
    store::LookupMap,
    test_utils::test_env::{alice, bob},
    AccountId, PromiseOrValue, Timestamp,
};
use sweat_jar_model::{
    api::{ClaimApi, JarApi, ScoreApi, WithdrawApi},
    jar::DepositTicket,
    product::{Apy, FixedProductTerms, Product, ScoreBasedProductTerms, Terms},
    withdraw::WithdrawView,
    ProductId, Score, Timezone, TokenAmount, UDecimal, MS_IN_DAY, MS_IN_HOUR, MS_IN_YEAR, UTC,
};

use crate::{
    common::{
        test_data::{set_test_future_success, set_test_log_events},
        tests::{Context, TokenUtils},
    },
    jar::model::Jar,
    product::model::v1::InterestCalculator,
    score::AccountScore,
    test_utils::admin,
    StorageKey,
};

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn record_score_by_non_manager() {
    let mut context = Context::new(admin());

    context.switch_account(alice());
    context.contract().record_score(vec![(alice(), vec![(100, 0.into())])]);
}

/// 12% jar should have the same interest as 12_000 score jar walking to the limit every day
/// Also this method tests score cap
#[test]
fn same_interest_in_score_jar_as_in_const_jar() {
    set_test_log_events(false);

    let term_in_days: u64 = 365;
    let term_in_ms: u64 = term_in_days * MS_IN_DAY;
    let half_period: u64 = term_in_days / 2;

    let regular_product = Product {
        id: "regular_product".to_string(),
        terms: Terms::Fixed(FixedProductTerms {
            lockup_term: term_in_ms.into(),
            apy: Apy::Constant(UDecimal::new(12000, 5)),
        }),
        ..Product::default()
    };

    let score_product = Product {
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: term_in_ms.into(),
            score_cap: 12_000,
        }),
        ..generate_score_based_product()
    };

    let mut context = Context::new(admin()).with_products(&[regular_product.clone(), score_product.clone()]);
    context.deposit(&alice(), &regular_product.id, 100.to_otto());
    context.deposit_with_timezone(&alice(), &score_product.id, 100.to_otto(), Timezone::hour_shift(3));

    assert_eq!(context.contract().get_timezone(alice()), Some(I64(10800000)));

    // Difference of 1 is okay because the missing otto-sweat is stored in claim remainder
    // and will eventually be added to total claimed balance
    fn compare_interest(context: &Context, regular_product_id: &ProductId, score_product_id: &ProductId) {
        let regular_interest = context.interest(&alice(), regular_product_id);
        let score_interest = context.interest(&alice(), score_product_id);
        let diff = regular_interest.abs_diff(score_interest);

        assert!(diff <= 1, "Diff is too big {diff}");
    }

    for day in 0..term_in_days {
        let now = day * MS_IN_DAY;
        context.set_block_timestamp_in_ms(now);
        context.record_score(&alice(), UTC(day * MS_IN_DAY), 20_000);

        compare_interest(&context, &regular_product.id, &score_product.id);

        if day == half_period {
            let jar_interest = context.interest(&alice(), &regular_product.id);
            let score_interest = context.interest(&alice(), &score_product.id);

            let claimed = context.claim_total(&alice());
            assert_eq!(claimed, jar_interest + score_interest);
        }
    }

    assert_eq!(
        context.jar(&alice(), &regular_product.id).cache.unwrap().updated_at,
        half_period * MS_IN_DAY
    );
    assert_eq!(
        context.jar(&alice(), &score_product.id).cache.unwrap().updated_at,
        (term_in_days - 1) * MS_IN_DAY
    );
}

#[test]
fn score_jar_claim_often_vs_claim_at_the_end() {
    set_test_log_events(false);

    let term_in_days = 365;
    let term_in_ms = term_in_days * MS_IN_DAY;

    let product = Product {
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: term_in_ms.into(),
            score_cap: 20_000,
        }),
        ..generate_score_based_product()
    };

    let mut context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(
            &alice(),
            &[(product.id.clone(), Jar::new().with_deposit(0, 100.to_otto()))],
        )
        .with_jars(
            &bob(),
            &[(product.id.clone(), Jar::new().with_deposit(0, 100.to_otto()))],
        );
    context.contract().get_account_mut(&alice()).score = AccountScore::new(Timezone::hour_shift(0));
    context.contract().get_account_mut(&bob()).score = AccountScore::new(Timezone::hour_shift(0));

    fn update_and_check(day: u64, context: &mut Context, total_claimed_bob: &mut u128, product_id: &ProductId) {
        let score: Score = (0..1000).fake();

        context.switch_account(admin());
        context.record_score(&alice(), UTC(day * MS_IN_DAY), score);
        context.record_score(&bob(), UTC(day * MS_IN_DAY), score);

        if day > 1 {
            context.switch_account(admin());
            context.record_score(&alice(), UTC((day - 1) * MS_IN_DAY), score);
            context.record_score(&bob(), UTC((day - 1) * MS_IN_DAY), score);
        }

        *total_claimed_bob += context.claim_total(&bob());
        assert_eq!(context.interest(&alice(), product_id), *total_claimed_bob, "{day}");
    }

    let mut total_claimed_bob: u128 = 0;

    // Update each hour for 10 days
    for hour in 0..(24 * 10) {
        context.set_block_timestamp_in_hours(hour);
        update_and_check(hour / 24, &mut context, &mut total_claimed_bob, &product.id);
    }

    // Update each day until 100 days has passed
    for day in 10..100 {
        context.set_block_timestamp_in_days(day);
        update_and_check(day, &mut context, &mut total_claimed_bob, &product.id);
    }

    total_claimed_bob += context.claim_total(&bob());

    assert_eq!(context.interest(&alice(), &product.id), total_claimed_bob);
    assert_eq!(context.claim_total(&alice()), total_claimed_bob);

    assert_eq!(
        context.jar(&alice(), &product.id).cache.unwrap().updated_at,
        MS_IN_DAY * 99
    );
}

#[test]
fn interest_does_not_increase_with_no_score() {
    set_test_log_events(false);

    let term_in_days = 365;
    let term_in_ms = term_in_days * MS_IN_DAY;

    let product = Product {
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: term_in_ms.into(),
            score_cap: 20_000,
        }),
        ..generate_score_based_product()
    };

    let mut context = Context::new(admin()).with_products(&[product.clone()]).with_jars(
        &alice(),
        &[(product.id.clone(), Jar::new().with_deposit(0, 100_000_000))],
    );
    context.contract().get_account_mut(&alice()).score = AccountScore::new(Timezone::hour_shift(0));

    context.set_block_timestamp_in_days(5);

    context.record_score(&alice(), UTC(5 * MS_IN_DAY), 1000);

    assert_eq!(context.interest(&alice(), &product.id), 0);

    context.set_block_timestamp_in_days(6);

    let interest_for_one_day = context.interest(&alice(), &product.id);
    assert_ne!(interest_for_one_day, 0);

    context.set_block_timestamp_in_days(7);
    assert_eq!(interest_for_one_day, context.interest(&alice(), &product.id));

    context.set_block_timestamp_in_days(50);
    assert_eq!(interest_for_one_day, context.interest(&alice(), &product.id));

    context.set_block_timestamp_in_days(100);
    assert_eq!(interest_for_one_day, context.interest(&alice(), &product.id));
}

#[test]
fn withdraw_score_jar() {
    set_test_log_events(false);

    let term_in_days = 7;
    let term_in_ms = term_in_days * MS_IN_DAY;

    let product = Product {
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: term_in_ms.into(),
            score_cap: 20_000,
        }),
        ..generate_score_based_product()
    };

    let mut context = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), Jar::new().with_deposit(0, 100))])
        .with_jars(&bob(), &[(product.id.clone(), Jar::new().with_deposit(0, 100))]);
    context.contract().get_account_mut(&alice()).score = AccountScore::new(Timezone::hour_shift(0));
    context.contract().get_account_mut(&bob()).score = AccountScore::new(Timezone::hour_shift(0));

    for i in 0..=10 {
        context.set_block_timestamp_in_days(i);

        context.record_score(&alice(), (i * MS_IN_DAY).into(), 1000);
        context.record_score(&bob(), (i * MS_IN_DAY).into(), 1000);

        if i == 5 {
            let claimed_alice = context.claim_total(&alice());
            let claimed_bob = context.claim_total(&bob());
            assert_eq!(claimed_alice, claimed_bob);
        }
    }

    // Alice claims first and then withdraws
    let claimed_alice = context.claim_total(&alice());
    let withdrawn_alice = context.withdraw(&alice(), &product.id);

    assert_eq!(context.claim_total(&alice()), 0);

    // Bob withdraws first and then claims
    context.switch_account(bob());
    let withdrawn_bob = context.withdraw(&bob(), &product.id);
    let claimed_bob = context.claim_total(&bob());

    assert_eq!(context.claim_total(&bob()), 0);

    assert_eq!(claimed_alice, claimed_bob);
    assert_eq!(withdrawn_alice, withdrawn_bob);

    // All jars were closed and deleted after full withdraw and claim
    assert!(context.contract().get_jars_for_account(alice()).is_empty());
    assert!(context.contract().get_jars_for_account(bob()).is_empty());
}

#[test]
fn revert_scores_on_failed_claim() {
    set_test_log_events(false);

    let term_in_days = 10;
    let term_in_ms = term_in_days * MS_IN_DAY;

    let product = Product {
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: term_in_ms.into(),
            score_cap: 20_000,
        }),
        ..generate_score_based_product()
    };

    let mut context = Context::new(admin()).with_products(&[product.clone()]).with_jars(
        &alice(),
        &[(product.id.clone(), Jar::new().with_deposit(0, 100_000_000))],
    );
    context.contract().get_account_mut(&alice()).score = AccountScore::new(Timezone::hour_shift(0));

    for day in 0..=term_in_days {
        context.set_block_timestamp_in_days(day);

        context.record_score(&alice(), (day * MS_IN_DAY).into(), 500);
        if day > 1 {
            context.record_score(&alice(), ((day - 1) * MS_IN_DAY).into(), 1000);
        }

        // Clear accounts cache to test deserialization
        if day == 3 {
            context.contract().accounts.flush();
            context.contract().accounts = LookupMap::new(StorageKey::Accounts);
        }

        // Normal claim. Score should change:
        if day == 4 {
            assert_eq!(context.score(&alice()).scores(), (500, 1000));
            assert_ne!(context.claim_total(&alice()), 0);
            assert_eq!(context.score(&alice()).scores(), (500, 0));
        }

        // Failed claim. Score should stay the same:
        if day == 8 {
            set_test_future_success(false);
            assert_eq!(context.score(&alice()).scores(), (500, 1000));
            assert_eq!(context.claim_total(&alice()), 0);
            assert_eq!(context.score(&alice()).scores(), (500, 1000));
        }
    }
}

impl Context {
    pub(crate) fn interest(&self, account_id: &AccountId, product_id: &ProductId) -> TokenAmount {
        let contract = self.contract();
        let product = &contract.get_product(product_id);
        let account = contract.get_account(account_id);
        let jar = account.get_jar(product_id);

        product.terms.get_interest(account, jar, self.now()).0
    }

    fn jar(&self, account_id: &AccountId, product_id: &ProductId) -> Jar {
        let contract = self.contract();
        let account = contract.get_account(account_id);

        account.get_jar(product_id).clone()
    }

    pub(crate) fn claim_total(&mut self, account_id: &AccountId) -> TokenAmount {
        self.switch_account(account_id);
        let PromiseOrValue::Value(claim_result) = self.contract().claim_total(None) else {
            panic!("Expected value");
        };

        claim_result.get_total().0
    }

    pub(crate) fn record_score(&mut self, account_id: &AccountId, time: UTC, score: Score) {
        self.switch_account(admin());
        self.contract()
            .record_score(vec![(account_id.clone(), vec![(score, time)])]);
    }

    pub(crate) fn withdraw(&mut self, account_id: &AccountId, product_id: &ProductId) -> WithdrawView {
        self.switch_account(account_id);
        let result = self.contract().withdraw(product_id.clone());

        match result {
            PromiseOrValue::Promise(_) => {
                panic!("Expected value");
            }
            PromiseOrValue::Value(value) => value,
        }
    }

    pub(crate) fn score(&self, account_id: &AccountId) -> AccountScore {
        self.contract().get_account(account_id).score
    }
}

#[test]
fn timestamps() {
    const BASE_TIME: Timestamp = 1729692817027;
    const TEST_TIME: Timestamp = 1729694971000;

    set_test_log_events(false);

    let product = generate_score_based_product();
    let mut ctx = Context::new(admin()).with_products(&[product.clone()]);

    ctx.set_block_timestamp_in_ms(BASE_TIME);
    ctx.switch_account(admin());
    ctx.contract().deposit(
        alice(),
        DepositTicket {
            product_id: product.id.clone(),
            valid_until: (BASE_TIME + MS_IN_YEAR).into(),
            timezone: Some(Timezone::hour_shift(4)),
        },
        100_000_000.to_otto(),
        &None,
    ); // Wed Oct 23 2024 14:13:37

    ctx.set_block_timestamp_in_ms(TEST_TIME);
    ctx.record_score(&alice(), UTC(1729592064000), 8245);

    assert_eq!(
        22589041095890410958904,
        ctx.contract().get_total_interest(alice()).amount.total.0
    );

    for i in 0..100 {
        ctx.set_block_timestamp_in_ms(TEST_TIME + MS_IN_HOUR * i);

        assert_eq!(
            22589041095890410958904,
            ctx.contract().get_total_interest(alice()).amount.total.0
        );
    }
}

#[test]
fn test_steps_history() {
    const BASE_TIME: Timestamp = 1729692817027;

    set_test_log_events(false);

    let product = generate_score_based_product();
    let mut ctx = Context::new(admin()).with_products(&[product.clone()]).with_jars(
        &alice(),
        &[(product.id.clone(), Jar::new().with_deposit(BASE_TIME, 100))],
    );
    ctx.contract().get_account_mut(&alice()).score.timezone = Timezone::hour_shift(4);

    let check_score_interest = |ctx: &Context, val: u128| {
        assert_eq!(ctx.contract().get_score(alice()), Some(U128(val)));
    };

    ctx.set_block_timestamp_in_ms(BASE_TIME);

    check_score_interest(&ctx, 0);

    ctx.record_score(&alice(), UTC(BASE_TIME - MS_IN_DAY), 8245);

    check_score_interest(&ctx, 8245);

    ctx.set_block_timestamp_in_ms(BASE_TIME + MS_IN_DAY);

    check_score_interest(&ctx, 0);

    ctx.set_block_timestamp_in_ms(BASE_TIME + MS_IN_DAY * 10);

    check_score_interest(&ctx, 0);

    ctx.record_score(&alice(), UTC(BASE_TIME + MS_IN_DAY * 10), 10000);
    ctx.record_score(&alice(), UTC(BASE_TIME + MS_IN_DAY * 10), 101);
    ctx.record_score(&alice(), UTC(BASE_TIME + MS_IN_DAY * 9), 9000);
    ctx.record_score(&alice(), UTC(BASE_TIME + MS_IN_DAY * 9), 90);

    check_score_interest(&ctx, 9090);

    ctx.set_block_timestamp_in_ms(BASE_TIME + MS_IN_DAY * 11);

    check_score_interest(&ctx, 10101);

    ctx.set_block_timestamp_in_ms(BASE_TIME + MS_IN_DAY * 12);

    check_score_interest(&ctx, 0);
}

#[test]
fn record_max_score() {
    set_test_log_events(false);

    let product = generate_score_based_product();
    let mut ctx = Context::new(admin())
        .with_products(&[product.clone()])
        .with_jars(&alice(), &[(product.id.clone(), Jar::new().with_deposit(0, 100))]);
    ctx.contract().get_account_mut(&alice()).score.timezone = Timezone::hour_shift(4);

    ctx.record_score(&alice(), UTC(0), 25000);
    ctx.record_score(&alice(), UTC(0), 25000);
    ctx.record_score(&alice(), UTC(0), 25000);
    ctx.record_score(&alice(), UTC(0), 25000);

    ctx.set_block_timestamp_in_days(1);

    assert_eq!(ctx.contract().get_score(alice()).unwrap().0, 65535);
}

fn generate_score_based_product() -> Product {
    Product {
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            score_cap: 20_000,
            lockup_term: (10 * MS_IN_DAY).into(),
        }),
        ..Product::default()
    }
}

impl Context {
    fn deposit(&mut self, account_id: &AccountId, product_id: &ProductId, amount: TokenAmount) {
        self.deposit_internal(account_id, product_id, amount, None);
    }

    fn deposit_with_timezone(
        &mut self,
        account_id: &AccountId,
        product_id: &ProductId,
        amount: TokenAmount,
        timezone: Timezone,
    ) {
        self.deposit_internal(account_id, product_id, amount, Some(timezone));
    }

    fn deposit_internal(
        &mut self,
        account_id: &AccountId,
        product_id: &ProductId,
        amount: TokenAmount,
        timezone: Option<Timezone>,
    ) {
        self.switch_account(admin());
        self.contract().deposit(
            account_id.clone(),
            DepositTicket {
                product_id: product_id.clone(),
                valid_until: (self.now() + MS_IN_YEAR).into(),
                timezone,
            },
            amount,
            &None,
        );
    }
}

#[test]
fn claim_when_there_were_no_walkchains_for_some_time() {
    set_test_log_events(false);

    let product = Product {
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            score_cap: 18_000,
            lockup_term: (7 * MS_IN_DAY).into(),
        }),
        ..generate_score_based_product()
    };

    let mut ctx = Context::new(admin()).with_products(&[product.clone()]);

    ctx.switch_account(admin());
    ctx.set_block_timestamp_in_ms(1732653318018 - MS_IN_DAY);
    ctx.contract().deposit(
        alice(),
        DepositTicket {
            product_id: product.id.clone(),
            valid_until: (1733139450015 + MS_IN_YEAR).into(),
            timezone: Some(Timezone::hour_shift(0)),
        },
        0,
        &None,
    );

    ctx.set_block_timestamp_in_ms(1732653318018);
    ctx.contract()
        .record_score(vec![(alice(), vec![(15100, 1732653318018.into())])]);

    ctx.set_block_timestamp_in_ms(1733139450015);
    ctx.contract().deposit(
        alice(),
        DepositTicket {
            product_id: product.id.clone(),
            valid_until: (1733139450015 + MS_IN_YEAR).into(),
            timezone: None,
        },
        100_000_000.to_otto(),
        &None,
    );

    ctx.set_block_timestamp_in_ms(1733140384365); // Mon Dec 02 2024 11:53:04

    assert_eq!(0, ctx.contract().get_total_interest(alice()).amount.total.0);
}
