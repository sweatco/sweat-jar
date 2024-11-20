#![cfg(test)]

use fake::Fake;
use near_sdk::{
    json_types::{I64, U128},
    store::LookupMap,
    test_utils::test_env::{alice, bob},
    NearToken, Timestamp,
};
use sweat_jar_model::{
    api::{JarApi, ProductApi, ScoreApi, WithdrawApi},
    jar::JarId,
    product::RegisterProductCommand,
    Score, Timezone, MS_IN_DAY, MS_IN_HOUR, UTC,
};

use crate::{
    common::{
        test_data::{set_test_future_success, set_test_log_events},
        tests::Context,
    },
    test_builder::{JarField, ProductField::*, TestAccess, TestBuilder},
    test_utils::{admin, expect_panic, UnwrapPromise, PRODUCT, SCORE_PRODUCT},
    StorageKey,
};

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn record_score_by_non_manager() {
    let ctx = TestBuilder::new().build();
    ctx.contract().record_score(vec![(alice(), vec![(100, 0.into())])]);
}

#[test]
fn create_invalid_step_product() {
    let mut ctx = TestBuilder::new().build();

    let mut command = RegisterProductCommand {
        id: "aa".to_string(),
        apy_default: (10.into(), 3),
        apy_fallback: None,
        cap_min: Default::default(),
        cap_max: Default::default(),
        terms: Default::default(),
        withdrawal_fee: None,
        public_key: None,
        is_enabled: false,
        score_cap: 1000,
    };

    ctx.switch_account(admin());

    ctx.set_deposit_yocto(1);

    expect_panic(&ctx, "Step based products do not support constant APY", || {
        ctx.contract().register_product(command.clone());
    });

    command.apy_fallback = Some((10.into(), 3));

    expect_panic(&ctx, "Step based products do not support downgradable APY", || {
        ctx.contract().register_product(command);
    });
}

/// 12% jar should have the same interest as 12_000 score jar walking to the limit every day
/// Also this method tests score cap
#[test]
fn same_interest_in_score_jar_as_in_const_jar() {
    const JAR: JarId = 0;
    const SCORE_JAR: JarId = 1;

    const DAYS: u64 = 365;
    const HALF_PERIOD: u64 = DAYS / 2;

    set_test_log_events(false);

    let mut ctx = TestBuilder::new()
        .product(PRODUCT, APY(12))
        .jar(JAR, ())
        .product(SCORE_PRODUCT, [APY(0), ScoreCap(12_000)])
        .jar(SCORE_JAR, JarField::Timezone(Timezone::hour_shift(3)))
        .build();

    assert_eq!(ctx.contract().get_timezone(alice()), Some(I64(10800000)));

    // Difference of 1 is okay because the missing yoctosweat is stored in claim remainder
    // and will eventually be added to total claimed balance
    fn compare_interest(ctx: &Context) {
        let diff = ctx.interest(JAR) as i128 - ctx.interest(SCORE_JAR) as i128;
        assert!(diff <= 1, "Diff is too big {diff}");
    }

    let mut total_claimed = 0;

    for day in 0..DAYS {
        ctx.set_block_timestamp_in_days(day);

        ctx.record_score(UTC(day * MS_IN_DAY), 20_000, alice());

        compare_interest(&ctx);

        if day == HALF_PERIOD {
            let jar_interest = ctx.interest(JAR);
            let score_interest = ctx.interest(SCORE_JAR);

            let claimed = ctx.claim_total(alice());

            total_claimed += claimed;

            assert_eq!(claimed, jar_interest + score_interest);
        }
    }

    assert_eq!(ctx.jar(JAR).cache.unwrap().updated_at, HALF_PERIOD * MS_IN_DAY);
    assert_eq!(ctx.jar(SCORE_JAR).cache.unwrap().updated_at, (DAYS - 1) * MS_IN_DAY);

    compare_interest(&ctx);

    total_claimed += ctx.claim_total(alice());

    assert_eq!(
        total_claimed,
        // The difference here is because the step jars doesn't receive interest for the first day
        // Because there were no steps at -1 day
        // But trough entire staking period the values match for regular and for step jar
        NearToken::from_near(24).as_yoctonear() - 65_753_424_657_534_246_575_344
    );
}

#[test]
fn score_jar_claim_often_vs_claim_at_the_end() {
    const ALICE_JAR: JarId = 0;
    const BOB_JAR: JarId = 1;

    set_test_log_events(false);

    let mut ctx = TestBuilder::new()
        .product(SCORE_PRODUCT, [APY(0), ScoreCap(20_000)])
        .jar(ALICE_JAR, JarField::Timezone(Timezone::hour_shift(0)))
        .jar(
            BOB_JAR,
            [JarField::Account(bob()), JarField::Timezone(Timezone::hour_shift(0))],
        )
        .build();

    fn update_and_check(day: u64, ctx: &mut Context, total_claimed_bob: &mut u128) {
        let score: Score = (0..1000).fake();

        ctx.switch_account(admin());
        ctx.record_score(UTC(day * MS_IN_DAY), score, alice());
        ctx.record_score(UTC(day * MS_IN_DAY), score, bob());

        if day > 1 {
            ctx.switch_account(admin());
            ctx.record_score(UTC((day - 1) * MS_IN_DAY), score, alice());
            ctx.record_score(UTC((day - 1) * MS_IN_DAY), score, bob());
        }

        *total_claimed_bob += ctx.claim_total(bob());
        assert_eq!(ctx.interest(ALICE_JAR), *total_claimed_bob, "{day}");
    }

    let mut total_claimed_bob: u128 = 0;

    // Update each hour for 10 days
    for hour in 0..(24 * 10) {
        ctx.set_block_timestamp_in_hours(hour);
        update_and_check(hour / 24, &mut ctx, &mut total_claimed_bob);
    }

    // Update each day until 100 days has passed
    for day in 10..100 {
        ctx.set_block_timestamp_in_days(day);
        update_and_check(day, &mut ctx, &mut total_claimed_bob);
    }

    total_claimed_bob += ctx.claim_total(bob());

    assert_eq!(ctx.interest(ALICE_JAR), total_claimed_bob);
    assert_eq!(ctx.claim_total(alice()), total_claimed_bob);

    assert_eq!(ctx.jar(ALICE_JAR).cache.unwrap().updated_at, MS_IN_DAY * 99);
}

#[test]
fn interest_does_not_increase_with_no_steps() {
    const ALICE_JAR: JarId = 0;

    set_test_log_events(false);

    let mut ctx = TestBuilder::new()
        .product(SCORE_PRODUCT, [APY(0), ScoreCap(20_000)])
        .jar(ALICE_JAR, JarField::Timezone(Timezone::hour_shift(0)))
        .build();

    ctx.set_block_timestamp_in_days(5);

    ctx.record_score(UTC(5 * MS_IN_DAY), 1000, alice());

    assert_eq!(ctx.interest(ALICE_JAR), 0);

    ctx.set_block_timestamp_in_days(6);

    let interest_for_one_day = ctx.interest(ALICE_JAR);
    assert_ne!(interest_for_one_day, 0);

    ctx.set_block_timestamp_in_days(7);
    assert_eq!(interest_for_one_day, ctx.interest(ALICE_JAR));

    ctx.set_block_timestamp_in_days(50);
    assert_eq!(interest_for_one_day, ctx.interest(ALICE_JAR));

    ctx.set_block_timestamp_in_days(100);
    assert_eq!(interest_for_one_day, ctx.interest(ALICE_JAR));
}

#[test]
fn withdraw_score_jar() {
    const ALICE_JAR: JarId = 0;
    const BOB_JAR: JarId = 1;

    set_test_log_events(false);

    let mut ctx = TestBuilder::new()
        .product(SCORE_PRODUCT, [APY(0), TermDays(7), ScoreCap(20_000)])
        .jar(ALICE_JAR, JarField::Timezone(Timezone::hour_shift(0)))
        .jar(
            BOB_JAR,
            [JarField::Account(bob()), JarField::Timezone(Timezone::hour_shift(0))],
        )
        .build();

    for i in 0..=10 {
        ctx.set_block_timestamp_in_days(i);

        ctx.record_score((i * MS_IN_DAY).into(), 1000, alice());
        ctx.record_score((i * MS_IN_DAY).into(), 1000, bob());

        if i == 5 {
            let claimed_alice = ctx.claim_total(alice());
            let claimed_bob = ctx.claim_total(bob());
            assert_eq!(claimed_alice, claimed_bob);
        }
    }

    // Alice claims first and then withdraws
    ctx.switch_account(alice());
    let claimed_alice = ctx.claim_total(alice());
    let withdrawn_alice = ctx
        .contract()
        .withdraw(ALICE_JAR.into(), None)
        .unwrap()
        .withdrawn_amount
        .0;

    assert_eq!(ctx.claim_total(alice()), 0);

    // Bob withdraws first and then claims
    ctx.switch_account(bob());
    let withdrawn_bob = ctx
        .contract()
        .withdraw(BOB_JAR.into(), None)
        .unwrap()
        .withdrawn_amount
        .0;
    let claimed_bob = ctx.claim_total(bob());

    assert_eq!(ctx.claim_total(bob()), 0);

    assert_eq!(claimed_alice, claimed_bob);
    assert_eq!(withdrawn_alice, withdrawn_bob);

    // All jars were closed and deleted after full withdraw and claim
    assert!(ctx.contract().account_jars(&alice()).is_empty());
    assert!(ctx.contract().account_jars(&bob()).is_empty());
}

#[test]
fn revert_scores_on_failed_claim() {
    const ALICE_JAR: JarId = 0;

    set_test_log_events(false);

    let mut ctx = TestBuilder::new()
        .product(SCORE_PRODUCT, [APY(0), TermDays(10), ScoreCap(20_000)])
        .jar(ALICE_JAR, JarField::Timezone(Timezone::hour_shift(0)))
        .build();

    for day in 0..=10 {
        ctx.set_block_timestamp_in_days(day);

        ctx.record_score((day * MS_IN_DAY).into(), 500, alice());
        if day > 1 {
            ctx.record_score(((day - 1) * MS_IN_DAY).into(), 1000, alice());
        }

        // Clear accounts cache to test deserialization
        if day == 3 {
            ctx.contract().accounts.flush();
            ctx.contract().accounts = LookupMap::new(StorageKey::AccountsVersioned);
        }

        // Normal claim. Score should change:
        if day == 4 {
            assert_eq!(ctx.score(ALICE_JAR).scores(), (500, 1000));
            assert_ne!(ctx.claim_total(alice()), 0);
            assert_eq!(ctx.score(ALICE_JAR).scores(), (500, 0));
        }

        // Failed claim. Score should stay the same:
        if day == 8 {
            set_test_future_success(false);
            assert_eq!(ctx.score(ALICE_JAR).scores(), (500, 1000));
            assert_eq!(ctx.claim_total(alice()), 0);
            assert_eq!(ctx.score(ALICE_JAR).scores(), (500, 1000));
        }
    }
}

#[test]
fn timestamps() {
    const ALICE_JAR: JarId = 0;

    set_test_log_events(false);

    let mut ctx = TestBuilder::new()
        .product(SCORE_PRODUCT, [APY(0), TermDays(10), ScoreCap(20_000)])
        .jar(ALICE_JAR, JarField::Timezone(Timezone::hour_shift(4)))
        .build();

    ctx.contract()
        .accounts
        .get_mut(&alice())
        .unwrap()
        .jars
        .first_mut()
        .unwrap()
        .created_at = 1729692817027;

    ctx.set_block_timestamp_in_ms(1729694971000);

    ctx.record_score(UTC(1729592064000), 8245, alice());

    assert_eq!(
        ctx.contract().get_total_interest(alice()).amount.total.0,
        22589041095890410958904
    );

    for i in 0..100 {
        ctx.set_block_timestamp_in_ms(1729694971000 + MS_IN_HOUR * i);

        assert_eq!(
            ctx.contract().get_total_interest(alice()).amount.total.0,
            22589041095890410958904
        );
    }
}

#[test]
fn test_steps_history() {
    const ALICE_JAR: JarId = 0;
    const BASE_TIME: Timestamp = 1729692817027;

    set_test_log_events(false);

    let mut ctx = TestBuilder::new()
        .product(SCORE_PRODUCT, [APY(0), TermDays(10), ScoreCap(20_000)])
        .jar(ALICE_JAR, JarField::Timezone(Timezone::hour_shift(4)))
        .build();

    ctx.contract()
        .accounts
        .get_mut(&alice())
        .unwrap()
        .jars
        .first_mut()
        .unwrap()
        .created_at = BASE_TIME;

    let check_score_interest = |ctx: &Context, val: u128| {
        assert_eq!(ctx.contract().get_score_interest(alice()), Some(U128(val)));
    };

    ctx.set_block_timestamp_in_ms(BASE_TIME);

    check_score_interest(&ctx, 0);

    ctx.record_score(UTC(BASE_TIME - MS_IN_DAY), 8245, alice());

    check_score_interest(&ctx, 8245);

    ctx.set_block_timestamp_in_ms(BASE_TIME + MS_IN_DAY);

    check_score_interest(&ctx, 0);

    ctx.set_block_timestamp_in_ms(BASE_TIME + MS_IN_DAY * 10);

    check_score_interest(&ctx, 0);

    ctx.record_score(UTC(BASE_TIME + MS_IN_DAY * 10), 10000, alice());
    ctx.record_score(UTC(BASE_TIME + MS_IN_DAY * 10), 101, alice());
    ctx.record_score(UTC(BASE_TIME + MS_IN_DAY * 9), 9000, alice());
    ctx.record_score(UTC(BASE_TIME + MS_IN_DAY * 9), 90, alice());

    check_score_interest(&ctx, 9090);

    ctx.set_block_timestamp_in_ms(BASE_TIME + MS_IN_DAY * 11);

    check_score_interest(&ctx, 10101);

    ctx.set_block_timestamp_in_ms(BASE_TIME + MS_IN_DAY * 12);

    check_score_interest(&ctx, 0);
}

#[test]
fn record_max_score() {
    const ALICE_JAR: JarId = 0;

    set_test_log_events(false);

    let mut ctx = TestBuilder::new()
        .product(SCORE_PRODUCT, [APY(0), TermDays(10), ScoreCap(20_000)])
        .jar(ALICE_JAR, JarField::Timezone(Timezone::hour_shift(4)))
        .build();

    ctx.record_score(UTC(0), 25000, alice());
    ctx.record_score(UTC(0), 25000, alice());
    ctx.record_score(UTC(0), 25000, alice());
    ctx.record_score(UTC(0), 25000, alice());

    ctx.set_block_timestamp_in_days(1);

    assert_eq!(ctx.contract().get_score_interest(alice()).unwrap().0, 65535);
}
