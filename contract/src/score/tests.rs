#![cfg(test)]

use near_sdk::test_utils::test_env::alice;
use sweat_jar_model::{jar::JarId, Timezone, MS_IN_DAY};

use crate::{
    common::{test_data::set_test_log_events, tests::Context},
    test_builder::{JarField, ProductField::*, TestAccess, TestBuilder},
    test_utils::{PRODUCT, SCORE_PRODUCT},
};

/// 12% jar should have the same interest as 12_000 score jar
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

    // Difference of 1 is okay because the missing yoctosweat is stored in claim remainder
    // and will eventually be added to total claimed balance
    fn compare_interest(ctx: &Context) {
        let diff = ctx.interest(JAR) as i128 - ctx.interest(SCORE_JAR) as i128;
        assert!(diff <= 1, "Diff is too big {diff}");
    }

    for day in 0..DAYS {
        ctx.set_block_timestamp_in_days(day);
        ctx.record_score(day * MS_IN_DAY, 20_000, alice());

        compare_interest(&ctx);

        if day == HALF_PERIOD {
            let jar_interest = ctx.interest(JAR);
            let score_interest = ctx.interest(SCORE_JAR);

            let claimed = ctx.claim_total(alice());

            assert_eq!(claimed, jar_interest + score_interest);
        }
    }

    assert_eq!(ctx.jar(JAR).cache.unwrap().updated_at, HALF_PERIOD * MS_IN_DAY);
    assert_eq!(ctx.jar(SCORE_JAR).cache.unwrap().updated_at, (DAYS - 1) * MS_IN_DAY);
}
