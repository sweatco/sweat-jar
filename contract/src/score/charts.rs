#![cfg(test)]

use anyhow::Result;
use fake::Fake;
use itertools::Itertools;
use near_sdk::test_utils::test_env::{alice, bob};
use sweat_jar_model::{
    data::product::{
        test_utils::{DEFAULT_PRODUCT_NAME, DEFAULT_SCORE_PRODUCT_NAME},
        Apy, FixedProductTerms, Product, ScoreBasedProductTerms, Terms,
    },
    Score, Timezone, UDecimal, MS_IN_DAY, MS_IN_YEAR, UTC,
};

use crate::{
    common::{test_data::set_test_log_events, tests::Context},
    jar::model::Jar,
    test_utils::admin,
};

fn generate_regular_product() -> Product {
    Product {
        id: DEFAULT_PRODUCT_NAME.to_string(),
        terms: Terms::Fixed(FixedProductTerms {
            lockup_term: MS_IN_YEAR.into(),
            apy: Apy::Constant(UDecimal::new(12000, 5)),
        }),
        ..Product::default()
    }
}
fn generate_score_based_product() -> Product {
    Product {
        id: DEFAULT_SCORE_PRODUCT_NAME.to_string(),
        terms: Terms::ScoreBased(ScoreBasedProductTerms {
            lockup_term: MS_IN_YEAR.into(),
            score_cap: 20_000,
        }),
        ..Product::default()
    }
}

fn generate_year_data() -> (Vec<u128>, Vec<u128>) {
    set_test_log_events(false);

    let regular_product = generate_regular_product();
    let score_based_product = generate_score_based_product();
    let mut ctx = Context::new(admin())
        .with_products(&vec![regular_product.clone(), score_based_product.clone()])
        .with_jars(
            &alice(),
            &vec![
                (
                    regular_product.id.clone(),
                    Jar::new().with_deposit(0, 100 * 10u128.pow(18)),
                ),
                (
                    score_based_product.id.clone(),
                    Jar::new().with_deposit(0, 100 * 10u128.pow(18)),
                ),
            ],
        );
    ctx.contract().get_account_mut(&alice()).score.timezone = Timezone::hour_shift(3);

    let mut result = vec![];

    ctx.switch_account(admin());

    for day in 1..400 {
        ctx.set_block_timestamp_in_days(day);

        if day < 100 {
            ctx.record_score(&alice(), UTC(MS_IN_DAY * day), (4_000..10_000).fake());
        } else {
            ctx.record_score(&alice(), UTC(MS_IN_DAY * day), (15_000..20_000).fake());
        }

        result.push((
            ctx.interest(&alice(), &score_based_product.id),
            ctx.interest(&alice(), &regular_product.id),
        ));
    }

    result.into_iter().unzip()
}

#[test]
#[ignore]
fn plot_year() -> Result<()> {
    let (_score, _simple) = generate_year_data();

    // TODO: fix
    // visu dependency caused https://github.com/sweatco/sweat-jar/actions/runs/13550344429/job/37872207500?pr=124
    // on linux machines
    // render_chart(Graph {
    //     title: "Step Jars Interest",
    //     data: [&score, &simple],
    //     legend: ["Step Jar", "Simple Jar"],
    //     x_title: "Days",
    //     y_title: "Interest",
    //     output_file: "../docs/year_walk.png",
    //     ..Default::default()
    // })?;

    Ok(())
}

type WeekData = (Vec<u128>, Vec<u128>, Vec<u128>, Vec<u128>, Vec<u128>);

fn generate_first_week_data() -> WeekData {
    set_test_log_events(false);

    let product = generate_score_based_product();
    let mut ctx = Context::new(admin())
        .with_products(&vec![product.clone()])
        .with_jars(
            &alice(),
            &[(product.id.clone(), Jar::new().with_deposit(0, 100 * 10u128.pow(18)))],
        )
        .with_jars(
            &bob(),
            &[(product.id.clone(), Jar::new().with_deposit(0, 100 * 10u128.pow(18)))],
        );

    ctx.contract().get_account_mut(&alice()).score.timezone = Timezone::hour_shift(0);
    ctx.contract().get_account_mut(&bob()).score.timezone = Timezone::hour_shift(0);

    let mut result = vec![];
    let mut score_walked: u128;

    let mut total_claimed: u128 = 0;

    for hour in 0..(24 * 5) {
        let day = hour / 24;

        ctx.set_block_timestamp_in_hours(hour);

        let score: Score = (0..1000).fake();

        ctx.switch_account(admin());
        ctx.record_score(&alice(), UTC(day * MS_IN_DAY), score);
        ctx.record_score(&bob(), UTC(day * MS_IN_DAY), score);

        if day > 1 {
            ctx.record_score(&alice(), UTC((day - 1) * MS_IN_DAY), score);
            ctx.record_score(&bob(), UTC((day - 1) * MS_IN_DAY), score);
        }

        score_walked = u128::from(score);

        let (today, yesterday) = ctx.score(&alice()).scores();

        // if hour % 15 == 0 {
        let claimed = ctx.claim_total(&bob());
        total_claimed += claimed;
        // }

        result.push((
            score_walked,
            ctx.interest(&alice(), &product.id),
            total_claimed,
            today as u128,
            yesterday as u128,
        ));
    }

    let (today, yesterday) = ctx.score(&bob()).scores();

    let claimed = ctx.claim_total(&bob());
    total_claimed += claimed;

    result.push((
        0,
        ctx.interest(&alice(), &product.id),
        total_claimed,
        today as u128,
        yesterday as u128,
    ));

    result.into_iter().multiunzip()
}

#[test]
#[ignore]
fn plot_first_week() -> Result<()> {
    let (_score_walked, _interest_alice, _claimed, _today, _yesterday) = generate_first_week_data();

    // render_chart(Graph {
    //     title: "Step Jars First Week",
    //     data: [&score_walked, &interest_alice, &today, &yesterday, &claimed],
    //     legend: ["Steps Walked", "Interest Alice", "Today", "Yesterday", "Claimed"],
    //     x_title: "Hours",
    //     y_title: "Interest",
    //     output_file: "../docs/first_week.png",
    //     ..Default::default()
    // })?;

    Ok(())
}

#[test]
#[ignore]
fn plot_first_week_with_claim() -> Result<()> {
    let (score_walked, ideal_jar, real_jar, claimed_ideal, claimed_real) = generate_first_week_data();

    // TODO: fix
    // visu dependency caused https://github.com/sweatco/sweat-jar/actions/runs/13550344429/job/37872207500?pr=124
    // on linux machines
    // render_chart(Graph {
    //     title: "Step Jars First Week With Claim",
    //     data: [&score_walked, &ideal_jar, &real_jar, &claimed_ideal, &claimed_real],
    //     legend: ["Steps Walked", "Ideal jar", "Real Jar", "Claimed Ideal", "Claimed Real"],
    //     x_title: "Hours",
    //     y_title: "Interest",
    //     output_file: "../docs/first_week_claim.png",
    // })?;

    Ok(())
}
