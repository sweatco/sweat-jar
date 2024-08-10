#![cfg(test)]

use anyhow::Result;
use fake::Fake;
use itertools::Itertools;
use near_sdk::test_utils::test_env::alice;
use sweat_jar_model::{jar::JarId, Score, Timezone, MS_IN_DAY, UTC};
use visu::{render_chart, Graph};

use crate::{
    common::test_data::set_test_log_events,
    test_builder::{JarField, ProductField::*, TestAccess, TestBuilder},
    test_utils::{PRODUCT, SCORE_PRODUCT},
};

fn generate_year_data() -> (Vec<u128>, Vec<u128>) {
    const JAR: JarId = 0;
    const STEP_JAR: JarId = 1;

    set_test_log_events(false);

    let mut context = TestBuilder::new()
        .product(SCORE_PRODUCT, [APY(0), ScoreCap(20_000)])
        .jar(STEP_JAR, JarField::Timezone(Timezone::hour_shift(3)))
        .product(PRODUCT, APY(12))
        .jar(JAR, ())
        .build();

    let mut result = vec![];

    for day in 1..400 {
        context.set_block_timestamp_in_days(day.try_into().unwrap());

        if day < 100 {
            context.record_score(UTC(MS_IN_DAY * day), (4_000..10_000).fake(), alice());
        } else {
            context.record_score(UTC(MS_IN_DAY * day), (15_000..20_000).fake(), alice());
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

fn generate_first_week_data(_with_claim: bool) -> (Vec<u128>, Vec<u128>, Vec<u128>, Vec<u128>, Vec<u128>) {
    const JAR: JarId = 0;

    set_test_log_events(false);

    let mut ctx = TestBuilder::new()
        .product(SCORE_PRODUCT, [APY(0), ScoreCap(20_000)])
        .jar(JAR, JarField::Timezone(Timezone::hour_shift(0)))
        .build();

    let mut result = vec![];
    let mut score_walked: u128;

    let mut score_history = vec![];

    let claimed: u128 = 0;

    for hour in 0..(24 * 7) {
        let day = hour / 24;

        ctx.set_block_timestamp_in_hours(hour);

        let score: Score = (0..1000).fake();

        score_history.push(score);
        ctx.record_score(UTC(day * MS_IN_DAY), score, alice());

        if day > 1 {
            ctx.record_score(UTC((day - 1) * MS_IN_DAY), score, alice());
        }

        score_walked = u128::from(score);

        let (today, yesterday) = ctx.score(JAR);

        result.push((
            score_walked,
            ctx.interest(JAR),
            claimed,
            today as u128,
            yesterday as u128,
        ));
    }

    result.into_iter().multiunzip()
}

#[test]
#[ignore]
fn plot_first_week() -> Result<()> {
    let (score_walked, interest, _claimed, today, yesterday) = generate_first_week_data(false);

    render_chart(Graph {
        title: "Step Jars First Week",
        data: [&score_walked, &interest, &today, &yesterday],
        legend: ["Steps Walked", "Interest", "Today", "Yesterday"],
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
    // let (score_walked, ideal_jar, real_jar, claimed_ideal, claimed_real) = generate_first_week_data(true);
    //
    // render_chart(Graph {
    //     title: "Step Jars First Week With Claim",
    //     data: [&score_walked, &ideal_jar, &real_jar, &claimed_ideal, &claimed_real],
    //     legend: ["Steps Walked", "Ideal jar", "Real Jar", "Claimed Ideal", "Claimed Real"],
    //     x_title: "Hours",
    //     y_title: "Interest",
    //     output_file: "../docs/first_week_claim.png",
    //     ..Default::default()
    // })?;

    Ok(())
}
