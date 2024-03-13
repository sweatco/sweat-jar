#![cfg(test)]

use std::collections::HashMap;

use anyhow::Result;
use near_sdk::json_types::U128;
use near_workspaces::types::Gas;

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    measure::{
        measure::scoped_command_measure,
        utils::{add_jar, append_measure, generate_permutations, measure_jars_range, retry_until_ok, MeasureData},
    },
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn measure_top_up_test() -> Result<()> {
    async fn top_up() -> Result<()> {
        let measured = scoped_command_measure(
            generate_permutations(
                &[RegisterProductCommand::Locked10Minutes6PercentsTopUp],
                &measure_jars_range(),
            ),
            measure_top_up,
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

        append_measure("top_up", map)
    }

    retry_until_ok(top_up).await?;

    Ok(())
}

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn single_top_up() -> Result<()> {
    let gas = measure_top_up((RegisterProductCommand::Locked10Minutes6PercentsTopUp, 1)).await?;

    dbg!(&gas);

    Ok(())
}

#[mutants::skip]
async fn measure_top_up(input: (RegisterProductCommand, usize)) -> Result<Gas> {
    let (product, jars_count) = input;

    let mut context = prepare_contract(None, [product]).await?;

    let alice = context.alice().await?;

    for _ in 0..jars_count {
        add_jar(&context, &alice, product, 100_000).await?;
    }

    Ok(context
        .sweat_jar()
        .top_up(&alice, 1, U128(1_000), &context.ft_contract())
        .result()
        .await?
        .total_gas_burnt)
}
