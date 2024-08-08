#![cfg(test)]

use anyhow::Result;
use fake::Fake;
use itertools::Itertools;
use near_sdk::test_utils::test_env::{alice, bob};
use sweat_jar_model::{jar::JarId, MS_IN_DAY};
use visu::{render_chart, Graph};

use crate::{
    test_builder::{JarField::*, ProductField::*, TestAccess, TestBuilder},
    test_utils::{PRODUCT, SCORE_PRODUCT},
};

fn generate_year_data() -> (Vec<u128>, Vec<u128>) {
    const JAR: JarId = 0;
    const STEP_JAR: JarId = 1;

    let mut context = TestBuilder::new()
        .product(SCORE_PRODUCT, [APY(0), ScoreCap(20_000)])
        .jar(STEP_JAR, ())
        .product(PRODUCT, APY(12))
        .jar(JAR, ())
        .build();

    let mut result = vec![];

    for day in 1..400 {
        context.set_block_timestamp_in_days(day.try_into().unwrap());

        if day < 100 {
            context.record_score(MS_IN_DAY * day, (4_000..10_000).fake(), alice());
        } else {
            context.record_score(MS_IN_DAY * day, (15_000..20_000).fake(), alice());
        }

        result.push((context.interest(STEP_JAR), context.interest(JAR)));
    }

    result.into_iter().unzip()
}

#[test]
#[ignore]
fn plot_year() -> Result<()> {
    let (score, simple) = generate_year_data();

    render_chart(Graph {
        title: "Step Jars Interest",
        data: [&score, &simple],
        legend: ["Step Jar", "Simple Jar"],
        x_title: "Days",
        y_title: "Interest",
        output_file: "../docs/year_walk.png",
        ..Default::default()
    })?;

    Ok(())
}

fn generate_first_week_data(with_claim: bool) -> (Vec<u128>, Vec<u128>, Vec<u128>, Vec<u128>, Vec<u128>) {
    const IDEAL_JAR: JarId = 0;
    const REAL_JAR: JarId = 1;

    let mut ctx = TestBuilder::new()
        .product(SCORE_PRODUCT, [APY(0), ScoreCap(20_000)])
        .jar(IDEAL_JAR, Account(alice()))
        .jar(REAL_JAR, Account(bob()))
        .build();

    let walkchain_updates: Vec<i32> = (0..10).map(|day| day * 24 + (4..10).fake::<i32>()).collect();

    let mut result = vec![];
    let mut score_walked: u128 = 0;

    let mut score_history = vec![];

    let score_walked_data: &[u128] = &[5000, 10000, 25000, 10000, 20000, 10000, 5000];

    let mut claimed_ideal: u128 = 0;
    let mut claimed_real: u128 = 0;

    let mut walkchain_update_index = 0;

    for hour in 0..(24 * 7) {
        let day = hour / 24;

        if hour % 24 == 0 {
            score_history.push(score_walked);
            ctx.record_score(day * MS_IN_DAY, score_walked.try_into().unwrap(), alice());
            score_walked = 0;
        }

        score_walked += score_walked_data[day as usize] / 24;

        ctx.set_block_timestamp_in_hours(hour);

        // if walkchain_update_index > 0 {
        //     if with_claim && hour as i32 == walkchain_updates[walkchain_update_index - 1] + 4 {
        //         claimed_ideal += ctx.claim_total(alice());
        //         claimed_real += ctx.claim_total(bob());
        //
        //         assert_eq!(claimed_ideal, claimed_real);
        //     }
        // }

        if with_claim && hour as i32 == walkchain_updates[walkchain_update_index] - 2 {
            claimed_ideal += ctx.claim_total(alice());
            claimed_real += ctx.claim_total(bob());

            // assert_eq!(claimed_ideal, claimed_real);
        }

        // if with_claim && hour == (24 * 7) - 1 {
        //     claimed_ideal += ctx.claim_total(alice());
        //     claimed_real += ctx.claim_total(bob());
        //
        //     assert_eq!(claimed_ideal - claimed_real, 2);
        // }

        if hour as i32 == walkchain_updates[walkchain_update_index] {
            walkchain_updates[walkchain_update_index];
            walkchain_update_index += 1;

            ctx.record_score(day * MS_IN_DAY, score_history[day as usize].try_into().unwrap(), bob());
        }

        claimed_ideal += ctx.claim_total(alice());
        claimed_real += ctx.claim_total(bob());

        result.push((
            score_walked,
            ctx.interest(IDEAL_JAR),
            ctx.interest(REAL_JAR),
            claimed_ideal,
            claimed_real,
        ));
    }

    result.into_iter().multiunzip()
}

#[test]
#[ignore]
fn plot_first_week() -> Result<()> {
    let (score_walked, ideal_jar, real_jar, _claimed_ideal, _claimed_real) = generate_first_week_data(false);

    render_chart(Graph {
        title: "Step Jars First Week",
        data: [&score_walked, &ideal_jar, &real_jar],
        legend: ["Steps Walked", "Ideal jar", "Real Jar"],
        x_title: "Hours",
        y_title: "Interest",
        output_file: "../docs/first_week.png",
        ..Default::default()
    })?;

    Ok(())
}

#[test]
#[ignore]
fn plot_first_week_with_claim() -> Result<()> {
    let (score_walked, ideal_jar, real_jar, claimed_ideal, claimed_real) = generate_first_week_data(true);

    render_chart(Graph {
        title: "Step Jars First Week With Claim",
        data: [&score_walked, &ideal_jar, &real_jar, &claimed_ideal, &claimed_real],
        legend: ["Steps Walked", "Ideal jar", "Real Jar", "Claimed Ideal", "Claimed Real"],
        x_title: "Hours",
        y_title: "Interest",
        output_file: "../docs/first_week_claim.png",
        ..Default::default()
    })?;

    Ok(())
}
