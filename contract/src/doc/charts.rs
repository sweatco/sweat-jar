#![cfg(test)]

use anyhow::Result;
use fake::Fake;
use itertools::Itertools;
use near_sdk::AccountId;
use rstest::rstest;
use sweat_jar_model::{
    data::{jar::Jar, product::Product},
    Score, Timezone, MS_IN_DAY, UTC,
};

use crate::{
    common::{
        env::test_env_ext,
        testing::{accounts::*, Context, TokenUtils},
    },
    feature::{account::model::test_utils::jar, product::model::test_utils::*},
};

#[rstest]
#[ignore]
fn plot_year(
    admin: AccountId,
    alice: AccountId,
    #[from(product_1_year_12_percent)] regular_product: Product,
    #[from(product_1_year_20_cap_score_based)] score_based_product: Product,
    #[from(jar)]
    #[with(vec![(0, 100 * 10u128.to_otto())])]
    regular_jar: Jar,
    #[from(jar)]
    #[with(vec![(0, 100 * 10u128.to_otto())])]
    score_based_jar: Jar,
) -> Result<()> {
    test_env_ext::set_test_log_events(false);

    let mut score = vec![];
    let mut simple = vec![];

    let mut ctx = Context::new(admin)
        .with_products(&vec![regular_product.clone(), score_based_product.clone()])
        .with_jars(
            &alice,
            &vec![
                (regular_product.id.clone(), regular_jar),
                (score_based_product.id.clone(), score_based_jar),
            ],
        );
    ctx.contract().get_account_mut(&alice).score.timezone = Timezone::hour_shift(3);

    ctx.switch_account_to_manager();

    for day in 1..400 {
        ctx.set_block_timestamp_in_days(day);

        if day < 100 {
            ctx.record_score(&alice, UTC(MS_IN_DAY * day), (4_000..10_000).fake());
        } else {
            ctx.record_score(&alice, UTC(MS_IN_DAY * day), (15_000..20_000).fake());
        }

        score.push(ctx.interest(&alice, &score_based_product.id));
        simple.push(ctx.interest(&alice, &regular_product.id));
    }

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

#[rstest]
#[ignore]
fn plot_first_week(
    admin: AccountId,
    alice: AccountId,
    bob: AccountId,
    #[from(product_1_year_20_cap_score_based)] product: Product,
    #[with(vec![(0, 100 * 10u128.to_otto())])] jar: Jar,
) -> Result<()> {
    test_env_ext::set_test_log_events(false);

    let mut ctx = Context::new(admin)
        .with_products(&vec![product.clone()])
        .with_jars(&alice, &[(product.id.clone(), jar.clone())])
        .with_jars(&bob, &[(product.id.clone(), jar.clone())]);

    ctx.contract().get_account_mut(&alice).score.timezone = Timezone::hour_shift(0);
    ctx.contract().get_account_mut(&bob).score.timezone = Timezone::hour_shift(0);

    let mut result = vec![];
    let mut score_walked: u128;

    let mut total_claimed: u128 = 0;

    for hour in 0..(24 * 5) {
        let day = hour / 24;

        ctx.set_block_timestamp_in_hours(hour);

        let score: Score = (0..1000).fake();

        ctx.switch_account_to_manager();
        ctx.record_score(&alice, UTC(day * MS_IN_DAY), score);
        ctx.record_score(&bob, UTC(day * MS_IN_DAY), score);

        if day > 1 {
            ctx.record_score(&alice, UTC((day - 1) * MS_IN_DAY), score);
            ctx.record_score(&bob, UTC((day - 1) * MS_IN_DAY), score);
        }

        score_walked = u128::from(score);

        let (today, yesterday) = ctx.score(&alice).scores();

        // if hour % 15 == 0 {
        let claimed = ctx.claim_total(&bob);
        total_claimed += claimed;
        // }

        result.push((
            score_walked,
            ctx.interest(&alice, &product.id),
            total_claimed,
            today as u128,
            yesterday as u128,
        ));
    }

    let (today, yesterday) = ctx.score(&bob).scores();

    let claimed = ctx.claim_total(&bob);
    total_claimed += claimed;

    result.push((
        0,
        ctx.interest(&alice, &product.id),
        total_claimed,
        today as u128,
        yesterday as u128,
    ));

    let (_score_walked, _interest_alice, _claimed, _today, _yesterday): WeekData = result.into_iter().multiunzip();

    // render_chart(Graph {
    //     title: "Step Jars First Week",
    //     data: [&score_walked, &interest_alice, &today, &yesterday, &claimed],
    //     legend: ["Steps Walked", "Interest Alice", "Today", "Yesterday", "Claimed"],
    //     x_title: "Hours",
    //     y_title: "Interest",
    //     output_file: "../docs/first_week.png",
    //     ..Default::default()
    // })?;

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
