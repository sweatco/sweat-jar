//! Lightweight gas-measurement tests.
//!
//! These are `#[ignore]`d so a plain `cargo test`/CI run skips them; run them
//! explicitly via `make measure-gas`. Their printed TGas figures show how the
//! `claim_total`/`withdraw_all` callbacks scale with the number of jars.

mod common;

use anyhow::Result;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand};
use near_workspaces::types::Gas;

fn report(label: &str, samples: &[(usize, Gas)]) {
    println!("\n=== {label} ===");
    for &(count, gas) in samples {
        println!("{count:>5} jars: {:>8.3} TGas", gas.as_gas() as f64 / 1e12);
    }
    for pair in samples.windows(2) {
        let (count_a, gas_a) = pair[0];
        let (count_b, gas_b) = pair[1];
        let diff = gas_b.as_gas() as i128 - gas_a.as_gas() as i128;
        let per_step = diff as f64 / (count_b - count_a) as f64 / 1e9;
        println!(
            "  {count_a:>5} -> {count_b:>5}: {:+.3} TGas total, {per_step:+.3} GGas/step",
            diff as f64 / 1e12
        );
    }
}

const JAR_COUNTS: [usize; 4] = [1, 50, 100, 200];

/// Measures `claim_total`'s gas cost (dominated by the `after_claim` callback)
/// as a function of the number of jars claimed at once.
#[tokio::test]
#[ignore = "run explicitly via `make measure-gas`"]
async fn measure_claim_gas() -> Result<()> {
    let product = RegisterProductCommand::Locked5Minutes60000Percents;
    let product_id = product.id();
    let mut samples = Vec::new();

    for &jars_count in &JAR_COUNTS {
        let context = prepare_contract([product]).await?;

        jar::bulk_create_jars(
            &context.jar,
            &context.manager,
            context.alice.id(),
            &product_id,
            100_000,
            jars_count as u16,
        )
        .await?;

        context.fast_forward_minutes(6).await?;

        let outcome = jar::claim_total_raw(&context.jar, &context.alice, None).await?;
        let gas = outcome.total_gas_burnt;
        outcome.into_result()?;

        samples.push((jars_count, gas));
    }

    report("claim_total (after_claim callback)", &samples);

    Ok(())
}

/// Measures `withdraw_all`'s gas cost (dominated by the `after_bulk_withdraw`
/// callback) as a function of the number of mature jars withdrawn at once.
#[tokio::test]
#[ignore = "run explicitly via `make measure-gas`"]
async fn measure_withdraw_all_gas() -> Result<()> {
    let product = RegisterProductCommand::Locked5Minutes60000Percents;
    let product_id = product.id();
    let mut samples = Vec::new();

    for &jars_count in &JAR_COUNTS {
        let context = prepare_contract([product]).await?;

        context
            .bulk_create_jars(&context.alice, &product_id, 100_000, jars_count as u16)
            .await?;

        context.fast_forward_minutes(6).await?;
        jar::claim_total(&context.jar, &context.alice, None).await?;

        let outcome = context
            .alice
            .call(context.jar.id(), "withdraw_all")
            .max_gas()
            .transact()
            .await?;
        let gas = outcome.total_gas_burnt;
        outcome.into_result()?;

        samples.push((jars_count, gas));
    }

    report("withdraw_all (after_bulk_withdraw callback)", &samples);

    Ok(())
}
