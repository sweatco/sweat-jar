#![cfg(test)]

use fake::Fake;
use near_sdk::test_utils::test_env::alice;
use sweat_jar_model::{jar::JarId, Score};

use crate::{
    test_builder::{ProductField::*, TestAccess, TestBuilder},
    test_utils::{PRODUCT, SCORE_PRODUCT},
};

/// 12% jar should have the same interest as 6% + 1_000 score jar if user has 6_000 scores each period
/// Also this method tests score cap
#[test]
fn same_interest_in_score_jar_as_in_const_jar() {
    const JAR: JarId = 0;
    const STEP_JAR: JarId = 1;

    let score_cap: Score = 6_000;

    let mut ctx = TestBuilder::new()
        .product(PRODUCT, APY(12))
        .jar(JAR, ())
        .product(SCORE_PRODUCT, [APY(6), ScoreCap(score_cap)])
        .jar(STEP_JAR, ())
        .build();

    let reset_period_days = 7;

    for day in 0..400 {
        ctx.set_block_timestamp_in_days(day);

        if day % reset_period_days == 0 {
            ctx.record_score(1, score_cap + (0..5_000).fake::<Score>(), alice());
        }
    }
}

#[test]
fn max_score_apy() {
    // Will never have principal from score
    const PRODUCT_ZERO_CAP: &str = "product_zero_cap";
    const JAR_ZERO_CAP: JarId = 0;

    // This product should get 2% APY if user has 14_000 scores each day
    const PRODUCT_7K_STEPS: &str = "product_7k_score";
    const JAR_7K: JarId = 1;

    // Control 2% APY product
    const PRODUCT_APY_2: &str = "product_apy_2";
    const JAR_APY_2: JarId = 2;

    let mut ctx = TestBuilder::new()
        .product(PRODUCT_ZERO_CAP, [APY(0), ScoreCap(0)])
        .jar(JAR_ZERO_CAP, ())
        .product(PRODUCT_7K_STEPS, [APY(0), NoScoreCap])
        .jar(JAR_7K, ())
        .product(PRODUCT_APY_2, APY(2))
        .jar(JAR_APY_2, ())
        .build();

    for day in 0..400 {
        ctx.set_block_timestamp_in_days(day);
    }
}

#[test]
fn test_non_linear_score() {}
