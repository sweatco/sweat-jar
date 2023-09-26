#![cfg(test)]

use workspaces::types::Gas;

use crate::{
    common::{prepare_contract, Prepared},
    measure::{outcome_storage::OutcomeStorage, utils::add_jar},
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
async fn one_restake() -> anyhow::Result<()> {
    let gas = measure_restake((RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee, 1)).await?;

    dbg!(&gas);

    Ok(())
}

pub(crate) async fn measure_restake(input: (RegisterProductCommand, usize)) -> anyhow::Result<Gas> {
    let (product, jars_count) = input;

    let Prepared {
        context,
        manager: _,
        alice,
        fee_account: _,
    } = prepare_contract([product]).await?;

    for _ in 0..jars_count {
        add_jar(&context, &alice, product, 100_000).await?;
    }

    context.fast_forward_hours(2).await?;

    let jars = context
        .jar_contract
        .get_jars_for_account(&alice)
        .await?
        .as_array()
        .unwrap();

    // let original_jar_id = jars.as_array().unwrap().get(0).unwrap().get_jar_id();

    //context.jar_contract.restake(&alice, )

    let (gas, _claimed) =
        OutcomeStorage::measure("interest_to_claim", &alice, context.jar_contract.claim_total(&alice)).await?;

    Ok(gas)
}
