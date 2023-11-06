#![cfg(test)]

use itertools::Itertools;
use near_workspaces::types::Gas;

use crate::{
    common::{prepare_contract, Prepared},
    measure::{measure::scoped_command_measure, outcome_storage::OutcomeStorage, utils::generate_permutations},
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn measure_withdraw_test() -> anyhow::Result<()> {
    let result = scoped_command_measure(
        generate_permutations(
            &[
                RegisterProductCommand::Locked10Minutes6PercentsWithPercentWithdrawFee,
                RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee,
            ],
            &[100_000, 200_000, 300_000, 400_000, 500_000],
        ),
        measure_one_withdraw,
    )
    .await?;

    dbg!(&result);

    let all_gas = result.into_iter().map(|res| res.1).collect_vec();

    dbg!(&all_gas);

    dbg!(all_gas.iter().max());
    dbg!(all_gas.iter().min());

    Ok(())
}

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn one_withdraw() -> anyhow::Result<()> {
    let gas = measure_one_withdraw((
        RegisterProductCommand::Locked10Minutes6PercentsWithPercentWithdrawFee,
        100_000,
    ))
    .await?;

    dbg!(&gas);

    Ok(())
}

#[mutants::skip]
async fn measure_one_withdraw(data: (RegisterProductCommand, u128)) -> anyhow::Result<Gas> {
    let (product, anmount) = data;

    let Prepared {
        context,
        manager: _,
        alice,
        fee_account: _,
    } = prepare_contract([product]).await?;

    context
        .jar_contract
        .create_jar(&alice, product.id(), anmount, context.ft_contract.account().id())
        .await?;

    context.fast_forward_hours(1).await?;

    dbg!(&alice);

    let (gas, _withdraw_result) = OutcomeStorage::measure_operation(
        "after_withdraw_internal",
        &alice,
        context.jar_contract.withdraw(&alice, 0.into()),
    )
    .await?;

    Ok(gas)
}
