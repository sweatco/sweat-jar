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
    let a: Vec<_> = (0..10)
        .into_iter()
        .map(|_| {
            spawn(measure_register_product(
                RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee,
            ))
        })
        .collect_vec();

    let res = join_all(a)
        .await
        .into_iter()
        .flatten()
        .map(Result::unwrap)
        .collect_vec();

    dbg!(res);

    Ok(())
}

async fn measure_register_product(command: RegisterProductCommand) -> anyhow::Result<Gas> {
    let Prepared {
        context,
        manager,
        alice: _,
        fee_account: _,
    } = prepare_contract([]).await?;

    context.jar_contract.register_product(&manager, command.json()).await?;

    Ok(OutcomeStorage::get_result(&manager, "register_product").gas_burnt)
}
