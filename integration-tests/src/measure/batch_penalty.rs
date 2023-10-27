#![cfg(test)]

use std::collections::HashMap;

use anyhow::Result;
use near_workspaces::types::Gas;

use crate::{
    common::{prepare_contract, Prepared},
    measure::{
        measure::scoped_command_measure,
        outcome_storage::OutcomeStorage,
        utils::{add_jar, append_measure, generate_permutations, measure_jars_range, retry_until_ok, MeasureData},
    },
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
async fn measure_batch_penalty_test() -> Result<()> {
    async fn batch_penalty() -> Result<()> {
        let measured = scoped_command_measure(
            generate_permutations(
                &[RegisterProductCommand::Flexible6Months6Percents],
                &measure_jars_range(),
            ),
            measure_batch_penalty,
        )
        .await?;

        let mut map: HashMap<RegisterProductCommand, Vec<(Gas, usize)>> = HashMap::new();

        for measure in measured {
            map.entry(measure.0 .0).or_default().push((measure.1, measure.0 .1));
        }

        let map: HashMap<RegisterProductCommand, _> = map
            .into_iter()
            .map(|(key, gas_cost)| {
                let mut differences: Vec<i128> = Vec::new();
                for i in 1..gas_cost.len() {
                    let diff = gas_cost[i].0.as_gas() as i128 - gas_cost[i - 1].0.as_gas() as i128;
                    differences.push(diff);
                }

                (key, MeasureData::new(gas_cost, differences))
            })
            .collect();

        append_measure("batch_penalty", map)
    }

    retry_until_ok(batch_penalty).await?;

    Ok(())
}

#[ignore]
#[tokio::test]
async fn single_batch_penalty() -> anyhow::Result<()> {
    let gas = measure_batch_penalty((RegisterProductCommand::Flexible6Months6Percents, 1)).await?;

    dbg!(&gas);

    Ok(())
}

async fn measure_batch_penalty(input: (RegisterProductCommand, usize)) -> anyhow::Result<Gas> {
    let (product, jars_count) = input;

    let Prepared {
        mut context,
        manager,
        alice,
        fee_account: _,
    } = prepare_contract([product]).await?;

    for _ in 0..jars_count {
        add_jar(&context, &alice, product, 100_000).await?;
    }

    let jars = context
        .jar_contract
        .get_jars_for_account(&alice)
        .await?
        .into_iter()
        .map(|j| j.id)
        .collect();

    let (gas, _) = OutcomeStorage::measure_total(
        &manager,
        context
            .jar_contract
            .batch_set_penalty(&manager, vec![(alice.id().clone(), jars)], true),
    )
    .await?;

    Ok(gas)
}
