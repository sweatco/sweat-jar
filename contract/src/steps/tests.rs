#![cfg(test)]

use fake::Fake;
use sweat_jar_model::{jar::JarId, Steps};

use crate::{
    test_builder::{ProductField::*, TestAccess, TestBuilder},
    test_utils::STEPS_PRODUCT,
};

/// 12% jar should have the same interest as 6% + 1_000 steps jar if user walks 6_000 steps each period
/// Also this method tests steps cap
#[test]
fn same_interest_in_step_jar_as_in_const_jar() {
    const JAR: JarId = 0;
    const STEP_JAR: JarId = 1;

    let steps_cap: Steps = 6_000;

    let mut ctx = TestBuilder::new()
        .product(12)
        .jar(JAR)
        .product_build(STEPS_PRODUCT, [APY(6), StepsCap(steps_cap)])
        .jar(STEP_JAR)
        .build();

    let reset_period_days = 7;

    for day in 0..400 {
        ctx.set_block_timestamp_in_days(day);

        if day % reset_period_days == 0 {
            ctx.record_steps(1, steps_cap + (0..5_000).fake::<Steps>());
        }
    }
}

#[test]
fn immediate_max_steps_apy() {
    // Will never have principal from steps
    const PRODUCT_ZERO_CAP: &str = "product_zero_cap";
    const JAR_ZERO_CAP: JarId = 0;

    // This product should get 2% APY if user walks 14_000 steps immediately after period start
    const PRODUCT_7K_STEPS: &str = "product_7k_steps";
    const JAR_7K: JarId = 1;

    // Control 2% APY product
    const PRODUCT_APY_2: &str = "product_apy_2";
    const JAR_APY_2: JarId = 2;

    let mut ctx = TestBuilder::new()
        .product_build(PRODUCT_ZERO_CAP, [APY(0), StepsCap(0)])
        .jar(JAR_ZERO_CAP)
        .product_build(PRODUCT_7K_STEPS, [APY(0), NoStepsCap])
        .jar(JAR_7K)
        .product_build(PRODUCT_APY_2, APY(2))
        .jar(JAR_APY_2)
        .build();

    for day in 0..400 {
        ctx.set_block_timestamp_in_days(day);
    }
}

#[test]
fn test_non_linear_steps() {}
