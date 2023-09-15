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
    let all = RegisterProductCommand::all()
        .iter()
        .map(|product| redundant_command_measure(|| measure_register_one_product(*product)))
        .collect_vec();

    let res = join_all(all).await.into_iter().map(Result::unwrap).collect_vec();

    dbg!(&res);

    Ok(())
}
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

    // Check if all commands have the same anmount of gas
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
