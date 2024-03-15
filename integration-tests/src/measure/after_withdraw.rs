#![cfg(test)]

use anyhow::Result;
use itertools::Itertools;
use near_workspaces::types::Gas;
use sweat_jar_model::{api::WithdrawApiIntegration, U32};

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    measure::{measure::scoped_command_measure, utils::generate_permutations},
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn measure_withdraw_test() -> Result<()> {
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
async fn measure_one_withdraw(data: (RegisterProductCommand, u128)) -> Result<Gas> {
    let (product, anmount) = data;

    let mut context = prepare_contract(None, [product]).await?;

    let alice = context.alice().await?;

    context
        .sweat_jar()
        .create_jar(&alice, product.id(), anmount, &context.ft_contract())
        .await?;

    context.fast_forward_hours(1).await?;

    Ok(context
        .sweat_jar()
        .withdraw(U32(0), None)
        .with_user(&alice)
        .result()
        .await?
        .total_gas_burnt)
}
