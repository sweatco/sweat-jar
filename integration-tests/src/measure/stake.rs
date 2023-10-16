#![cfg(test)]

use std::collections::HashMap;

use itertools::Itertools;
use workspaces::types::Gas;

use crate::{
    common::{prepare_contract, Prepared},
    measure::{
        measure::scoped_command_measure,
        outcome_storage::OutcomeStorage,
        utils::{add_jar, generate_permutations},
    },
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
async fn measure_stake_total_test() -> anyhow::Result<()> {
    use RegisterProductCommand::*;

    let measured = scoped_command_measure(
        generate_permutations(
            &[
                Locked10Minutes6Percents,
                Locked12Months12Percents,
                Locked6Months6Percents,
                Flexible6Months6Percents,
                Locked6Months6PercentsWithWithdrawFee,
            ],
            &(1..10).collect_vec(),
        ),
        measure_stake,
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
            let mut differences: Vec<i128> = Vec::new();
            for i in 1..gas_cost.len() {
                let diff = gas_cost[i] as i128 - gas_cost[i - 1] as i128;
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
async fn one_stake() -> anyhow::Result<()> {
    let gas = measure_stake((RegisterProductCommand::Locked10Minutes6Percents, 1)).await?;

    dbg!(&gas);

    Ok(())
}

pub(crate) async fn measure_stake(input: (RegisterProductCommand, usize)) -> anyhow::Result<Gas> {
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

    let (gas, _) = OutcomeStorage::measure_total(
        &alice,
        context
            .jar_contract
            .create_jar(&alice, product.id(), 100_000, context.ft_contract.account().id()),
    )
    .await?;

    Ok(gas)
}
