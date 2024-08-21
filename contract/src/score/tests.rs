#![cfg(test)]

use fake::Fake;
use near_sdk::{
    test_utils::test_env::{alice, bob},
    NearToken,
};
use sweat_jar_model::{api::ScoreApi, jar::JarId, Score, Timezone, MS_IN_DAY, UTC};

use crate::{
    common::{test_data::set_test_log_events, tests::Context},
    test_builder::{JarField, ProductField::*, TestAccess, TestBuilder},
    test_utils::{admin, PRODUCT, SCORE_PRODUCT},
};

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn record_score_by_non_manager() {
    let ctx = TestBuilder::new().build();
    ctx.contract().record_score(vec![(alice(), vec![(100, 0.into())])]);
}

/// 12% jar should have the same interest as 12_000 score jar walking to the limit every day
/// Also this method tests score cap
#[test]
fn same_interest_in_score_jar_as_in_const_jar() {
    const JAR: JarId = 0;
    const SCORE_JAR: JarId = 1;

    const DAYS: u64 = 400;
    const HALF_PERIOD: u64 = DAYS / 2;

    set_test_log_events(false);

    let mut ctx = TestBuilder::new()
        .product(PRODUCT, APY(12))
        .jar(JAR, ())
        .product(SCORE_PRODUCT, [APY(0), ScoreCap(12_000)])
        .jar(SCORE_JAR, JarField::Timezone(Timezone::hour_shift(3)))
        .build();

    // Difference of 1 is okay because the missing yoctosweat is stored in claim remainder
    // and will eventually be added to total claimed balance
    fn compare_interest(ctx: &Context) {
        let diff = ctx.interest(JAR) as i128 - ctx.interest(SCORE_JAR) as i128;
        assert!(diff <= 1, "Diff is too big {diff}");
    }

    let mut total_claimed = 0;

    for day in 0..DAYS {
        ctx.set_block_timestamp_in_days(day);

        ctx.switch_account(admin());
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

    assert_eq!(total_claimed, NearToken::from_near(24).as_yoctonear());
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
