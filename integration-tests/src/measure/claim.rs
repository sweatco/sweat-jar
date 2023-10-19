#![cfg(test)]

use std::collections::HashMap;

use anyhow::Result;
use workspaces::types::Gas;

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
async fn measure_claim_total_test() -> Result<()> {
    async fn claim() -> Result<()> {
        let measured = scoped_command_measure(
            generate_permutations(
                &[
                    RegisterProductCommand::Locked10Minutes6Percents,
                    RegisterProductCommand::Locked10Minutes6PercentsWithFixedWithdrawFee,
                    RegisterProductCommand::Locked10Minutes6PercentsWithPercentWithdrawFee,
                ],
                &measure_jars_range(),
            ),
            measure_claim,
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
                    let diff = gas_cost[i].0 as i128 - gas_cost[i - 1].0 as i128;
                    differences.push(diff);
                }

                (key, MeasureData::new(gas_cost, differences))
            })
            .collect();

        append_measure("claim", map)
    }

    retry_until_ok(claim).await?;

    Ok(())
}

#[ignore]
#[tokio::test]
async fn single_claim() -> anyhow::Result<()> {
    let gas = measure_claim((RegisterProductCommand::Locked10Minutes6Percents, 1)).await?;

    dbg!(&gas);

    Ok(())
}

async fn measure_claim(input: (RegisterProductCommand, usize)) -> anyhow::Result<Gas> {
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

    let (gas, _) = OutcomeStorage::measure_total(&alice, context.jar_contract.claim_total(&alice)).await?;

    Ok(gas)
}
