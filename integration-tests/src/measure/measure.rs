#![cfg(test)]

use std::future::Future;

use futures::future::join_all;
use itertools::Itertools;
use tokio::spawn;
use workspaces::types::Gas;

use crate::{
    measure::{claim_total::measure_after_claim_total, register_product::measure_register_product},
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
async fn measure_register_product_test() -> anyhow::Result<()> {
    let measure_register_product =
        scoped_command_measure(RegisterProductCommand::all(), measure_register_product).await?;
    dbg!(&measure_register_product);

    Ok(())
}

#[ignore]
#[tokio::test]
async fn measure_after_claim_total_test() -> anyhow::Result<()> {
    let measure_after_claim_total = scoped_command_measure(1..20, measure_after_claim_total).await?;
    dbg!(&measure_after_claim_total);

    for i in (1..measure_after_claim_total.len()).rev() {
        let diff = measure_after_claim_total[i].1 - measure_after_claim_total[i - 1].1;

        dbg!(&diff);
    }

    Ok(())
}

async fn scoped_command_measure<Input, Inputs, Command, Fut>(
    inputs: Inputs,
    mut command: Command,
) -> anyhow::Result<Vec<(Input, Gas)>>
where
    Input: Copy,
    Inputs: IntoIterator<Item = Input>,
    Fut: Future<Output = anyhow::Result<Gas>> + Send + 'static,
    Command: FnMut(Input) -> Fut + Copy,
{
    let inputs = inputs.into_iter().collect_vec();

    let all = inputs
        .iter()
        .map(|inp| redundant_command_measure(move || command(*inp)))
        .collect_vec();

    let res: Vec<_> = join_all(all).await.into_iter().collect::<anyhow::Result<_>>()?;

    let res = inputs.into_iter().zip(res.into_iter()).collect_vec();

    Ok(res)
}

/// This method runs the same command several times and checks if
/// all executions took the same anmount of gas
async fn redundant_command_measure<Fut>(mut command: impl FnMut() -> Fut) -> anyhow::Result<Gas>
where
    Fut: Future<Output = anyhow::Result<Gas>> + Send + 'static,
{
    let futures = (0..2).into_iter().map(|_| spawn(command())).collect_vec();
    let all_gas: Vec<Gas> = join_all(futures)
        .await
        .into_iter()
        .flatten()
        .collect::<anyhow::Result<_>>()?;

    let gas = all_gas.first().unwrap();

    assert!(all_gas.iter().all(|g| g == gas));

    Ok(*gas)
}
