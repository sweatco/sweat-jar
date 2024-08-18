#![cfg(test)]

use fake::Fake;
use near_sdk::{
    test_utils::test_env::{alice, bob},
    NearToken,
};
use sweat_jar_model::{jar::JarId, Score, Timezone, MS_IN_DAY, UTC};

use crate::{
    common::{test_data::set_test_log_events, tests::Context},
    test_builder::{JarField, ProductField::*, TestAccess, TestBuilder},
    test_utils::{admin, PRODUCT, SCORE_PRODUCT},
};

#[test]
#[should_panic(expected = "Can be performed only by admin")]
fn record_score_by_non_manager() {
    let mut ctx = TestBuilder::new().build();
    ctx.record_score(UTC(0), 0, alice());
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

    // Total claimed balance after a year from 2 jars of 12% - 24.
    // 1 missing yoctonear is stored in claim remainder
    assert_eq!(total_claimed, NearToken::from_near(24).as_yoctonear() - 1);
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

    let mut total_claimed_bob: u128 = 0;
    let mut _claim_interval_hours: u64 = (5..20).fake();

    // Update each hour for 400 days
    for hour in 0..(24 * 7) {
        let day = hour / 24;

        ctx.set_block_timestamp_in_hours(hour);

        let score: Score = (0..1000).fake();

        ctx.switch_account(admin());
        ctx.record_score(UTC(day * MS_IN_DAY), score, alice());
        ctx.record_score(UTC(day * MS_IN_DAY), score, bob());

        if day > 1 {
            ctx.switch_account(admin());
            ctx.record_score(UTC((day - 1) * MS_IN_DAY), score, alice());
            ctx.record_score(UTC((day - 1) * MS_IN_DAY), score, bob());
        }

        if hour == 24 * 3 {
            _claim_interval_hours = (5..20).fake();
            dbg!(ctx.interest(ALICE_JAR));
            dbg!(ctx.interest(BOB_JAR));
            let claimed = ctx.claim_total(bob());
            dbg!(&claimed);
            total_claimed_bob += claimed;
            println!();
            println!();
            println!();
            println!();
        }

        if hour == 24 * 5 {
            _claim_interval_hours = (5..20).fake();
            dbg!(ctx.interest(ALICE_JAR));
            dbg!(ctx.interest(BOB_JAR));
            let claimed = ctx.claim_total(bob());
            dbg!(&claimed);
            total_claimed_bob += claimed;
            dbg!(&total_claimed_bob);
            println!();
            println!();
            println!();
            println!();
        }
    }

    println!();
    println!();
    println!();
    println!();

    let claimed = ctx.claim_total(bob());
    total_claimed_bob += claimed;

    dbg!(&total_claimed_bob);
    dbg!(ctx.interest(ALICE_JAR));

    let claimed_alice = ctx.claim_total(alice());
    dbg!(&claimed_alice);
}
