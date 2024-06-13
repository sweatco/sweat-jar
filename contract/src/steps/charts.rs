#![cfg(test)]

use anyhow::Result;
use fake::Fake;
use itertools::Itertools;
use near_sdk::test_utils::test_env::{alice, bob};
use sweat_jar_model::{jar::JarId, MS_IN_DAY};
use visu::{render_chart, Graph};

use crate::{
    test_builder::{JarField::*, ProductField::*, TestAccess, TestBuilder},
    test_utils::{PRODUCT, STEPS_PRODUCT},
};

fn generate_year_data() -> (Vec<u128>, Vec<u128>) {
    const JAR: JarId = 0;
    const STEP_JAR: JarId = 1;

    let mut context = TestBuilder::new()
        .product(STEPS_PRODUCT, [APY(0), StepsCap(20_000)])
        .jar(STEP_JAR, ())
        .product(PRODUCT, APY(12))
        .jar(JAR, ())
        .build();

    let mut result = vec![];

    for day in 1..400 {
        context.set_block_timestamp_in_days(day.try_into().unwrap());

        if day < 100 {
            context.record_steps(MS_IN_DAY * day, (4_000..10_000).fake(), alice());
        } else {
            context.record_steps(MS_IN_DAY * day, (15_000..20_000).fake(), alice());
        }

        result.push((context.interest(STEP_JAR, alice()), context.interest(JAR, alice())));
    }

    result.into_iter().unzip()
}

#[test]
#[ignore]
fn plot_year() -> Result<()> {
    let (steps, simple) = generate_year_data();

    render_chart(Graph {
        title: "Step Jars Interest",
        data: [&steps, &simple],
        legend: ["Step Jar", "Simple Jar"],
        x_title: "Days",
        y_title: "Interest",
        output_file: "../docs/year_walk.png",
        ..Default::default()
    })?;

    Ok(())
}

fn generate_first_week_data() -> (Vec<u128>, Vec<u128>, Vec<u128>) {
    const IDEAL_JAR: JarId = 0;
    const REAL_JAR: JarId = 1;

    let mut context = TestBuilder::new()
        .product(STEPS_PRODUCT, [APY(0), StepsCap(20_000)])
        .jar(IDEAL_JAR, Account(alice()))
        .jar(REAL_JAR, Account(bob()))
        .build();

    let mut walkchain: u128 = 0;

    let mut walkchain_updates: Vec<i32> = (0..10).map(|day| day * 24 + (4..10).fake::<i32>()).collect();

    let mut result = vec![];
    let mut steps_walked: u128 = 0;

    let mut steps_history = vec![];

    for hour in 0..(24 * 7) {
        let day = hour / 24;

        if hour % 24 == 0 {
            steps_history.push(steps_walked);
            context.record_steps(day * MS_IN_DAY, steps_walked.try_into().unwrap(), alice());
            steps_walked = 0;
        }

        steps_walked += (0..2000).fake::<u128>();

        context.set_block_timestamp_in_hours(hour);

        if hour as i32 == walkchain_updates[0] {
            walkchain_updates.remove(0);

            walkchain = if walkchain == 1 { 0 } else { 1 };

            context.record_steps(day * MS_IN_DAY, steps_history[day as usize].try_into().unwrap(), bob());
        }

        result.push((
            steps_walked,
            context.interest(IDEAL_JAR, alice()),
            context.interest(REAL_JAR, bob()),
        ));
    }

    result.into_iter().multiunzip()
}

#[test]
#[ignore]
fn plot_first_week() -> Result<()> {
    let (steps_walked, ideal_jar, real_jar) = generate_first_week_data();

    render_chart(Graph {
        title: "Step Jars First Week",
        data: [&steps_walked, &ideal_jar, &real_jar],
        legend: ["Steps Walked", "Ideal jar", "Real Jar"],
        x_title: "Hours",
        y_title: "Interest",
        output_file: "../docs/first_week.png",
        ..Default::default()
    })?;

    Ok(())
}
