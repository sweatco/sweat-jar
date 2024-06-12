#![cfg(test)]

use fake::Fake;
use itertools::Itertools;
use sweat_jar_model::{jar::JarId, MS_IN_DAY};
use visu::render_chart;

use crate::{
    test_builder::{ProductField::*, TestAccess, TestBuilder},
    test_utils::STEPS_PRODUCT,
};

struct AccrualsData {
    steps: u128,
    simple: u128,
}

fn get_data() -> Vec<AccrualsData> {
    const JAR: JarId = 0;
    const STEP_JAR: JarId = 1;

    let mut context = TestBuilder::new()
        .product_build(STEPS_PRODUCT, [APY(0), StepsCap(20_000)])
        .jar(STEP_JAR)
        .product(12)
        .jar(JAR)
        .build();

    let mut result = vec![];

    for day in 1..400 {
        context.set_block_timestamp_in_days(day.try_into().unwrap());

        if day < 100 {
            context.record_steps(MS_IN_DAY * day, (4_000..10_000).fake());
        } else {
            context.record_steps(MS_IN_DAY * day, (15_000..20_000).fake());
        }

        result.push(AccrualsData {
            steps: context.interest(STEP_JAR),
            simple: context.interest(JAR),
        });
    }

    result
}

#[test]
#[ignore]
fn plot() -> anyhow::Result<()> {
    let (steps, simple): (Vec<u128>, Vec<u128>) = get_data()
        .into_iter()
        .map(|data| (data.steps, data.simple))
        .multiunzip();

    render_chart("Step jars interest", [&steps, &simple], "../docs/walk.png")?;

    Ok(())
}
