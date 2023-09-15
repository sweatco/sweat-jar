#![cfg(test)]

use std::future::Future;

use futures::future::join_all;
use itertools::Itertools;
use tokio::spawn;
use workspaces::types::Gas;

use crate::{
    common::{prepare_contract, Prepared},
    measure::outcome_storage::OutcomeStorage,
    product::RegisterProductCommand,
};

#[tokio::test]
async fn measure() -> anyhow::Result<()> {
    let res = scoped_command_measure(RegisterProductCommand::all(), measure_register_one_product).await?;

    dbg!(&res);

    Ok(())
}

async fn scoped_command_measure<Input, Command, Fut>(
    input: &[Input],
    mut command: Command,
) -> anyhow::Result<Vec<(&Input, Gas)>>
where
    Input: Copy,
    Fut: Future<Output = anyhow::Result<Gas>> + Send + 'static,
    Command: FnMut(Input) -> Fut + Copy,
{
    let all = input
        .iter()
        .map(|inp| redundant_command_measure(move || command(*inp)))
        .collect_vec();

    let res: Vec<_> = join_all(all).await.into_iter().collect::<anyhow::Result<_>>()?;

    let res = input.into_iter().zip(res.into_iter()).collect_vec();

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

async fn measure_register_one_product(command: RegisterProductCommand) -> anyhow::Result<Gas> {
    let Prepared {
        context,
        manager,
        alice: _,
        fee_account: _,
    } = prepare_contract([]).await?;

    OutcomeStorage::measure(
        "register_product",
        &manager,
        context.jar_contract.register_product(&manager, command.json()),
    )
    .await
}
