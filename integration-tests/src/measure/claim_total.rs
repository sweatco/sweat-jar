#![cfg(test)]

use std::collections::HashMap;

use itertools::Itertools;
use workspaces::{types::Gas, Account};

use crate::{
    common::{prepare_contract, Prepared},
    context::Context,
    measure::{measure::scoped_command_measure, outcome_storage::OutcomeStorage, utils::generate_permutations},
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
async fn measure_after_claim_total_test() -> anyhow::Result<()> {
    let measured = scoped_command_measure(
        generate_permutations(
            &[
                RegisterProductCommand::Locked12Months12Percents,
                RegisterProductCommand::Locked6Months6Percents,
                RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee,
            ],
            &(1..5).collect_vec(),
        ),
        measure_after_claim_total,
    )
    .await?;

    dbg!(&measured);

    let mut map: HashMap<RegisterProductCommand, Vec<Gas>> = HashMap::new();

    for measure in measured {
        map.entry(measure.0 .0).or_default().push(measure.1);
    }

    dbg!(&map);

    let map: HashMap<RegisterProductCommand, _> = map
        .into_iter()
        .map(|(key, gas_cost)| {
            let mut differences: Vec<Gas> = Vec::new();
            for i in 1..gas_cost.len() {
                let diff = gas_cost[i] - gas_cost[i - 1];
                differences.push(diff);
            }

            (key, (gas_cost, differences))
        })
        .collect();

    dbg!(&map);

    Ok(())
}

#[ignore]
#[tokio::test]
async fn one_after_claim() -> anyhow::Result<()> {
    let gas = measure_after_claim_total((RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee, 1)).await?;

    dbg!(&gas);

    Ok(())
}

pub(crate) async fn measure_after_claim_total(input: (RegisterProductCommand, usize)) -> anyhow::Result<Gas> {
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

    let (gas, _claimed) =
        OutcomeStorage::measure("interest_to_claim", &alice, context.jar_contract.claim_total(&alice)).await?;

    Ok(gas)
}

async fn add_jar(
    context: &Context,
    account: &Account,
    product: RegisterProductCommand,
    amount: u128,
) -> anyhow::Result<()> {
    context
        .jar_contract
        .create_jar(account, product.id(), amount, context.ft_contract.account().id())
        .await?;

    Ok(())
}
