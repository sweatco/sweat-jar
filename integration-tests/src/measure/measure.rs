#![cfg(test)]

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
        .map(|product| measure_register_product(*product))
        .collect_vec();

    let res = join_all(all).await.into_iter().map(Result::unwrap).collect_vec();

    dbg!(&res);

    Ok(())
}

async fn measure_register_product(
    command: RegisterProductCommand,
) -> anyhow::Result<(RegisterProductCommand, Vec<Gas>)> {
    let a: Vec<_> = (0..5)
        .into_iter()
        .map(|_| spawn(measure_register_one_product(command)))
        .collect_vec();

    let res = join_all(a)
        .await
        .into_iter()
        .flatten()
        .map(Result::unwrap)
        .collect_vec();

    Ok((command, res))
}

async fn measure_register_one_product(command: RegisterProductCommand) -> anyhow::Result<Gas> {
    let Prepared {
        context,
        manager,
        alice: _,
        fee_account: _,
    } = prepare_contract([]).await?;

    OutcomeStorage::measure(
        &manager,
        context.jar_contract.register_product(&manager, command.json()),
    )
    .await?;

    Ok(OutcomeStorage::get_result(&manager, "register_product").gas_burnt)
}
