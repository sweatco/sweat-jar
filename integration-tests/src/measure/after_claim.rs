#![cfg(test)]

use std::collections::HashMap;

use itertools::Itertools;
use near_workspaces::types::Gas;
use sweat_jar_model::api::ClaimApiIntegration;

use crate::{
    context::{prepare_contract, IntegrationContext},
    measure::{
        measure::scoped_command_measure,
        utils::{add_jar, generate_permutations},
    },
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
#[mutants::skip]
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
                let diff = gas_cost[i].as_gas() - gas_cost[i - 1].as_gas();
                differences.push(Gas::from_gas(diff));
            }

            (key, (gas_cost, differences))
        })
        .collect();

    dbg!(&map);

    Ok(())
}

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn one_after_claim() -> anyhow::Result<()> {
    let gas = measure_after_claim_total((RegisterProductCommand::Locked6Months6PercentsWithWithdrawFee, 1)).await?;

    dbg!(&gas);

    Ok(())
}

#[mutants::skip]
pub(crate) async fn measure_after_claim_total(input: (RegisterProductCommand, usize)) -> anyhow::Result<Gas> {
    let (product, jars_count) = input;

    let mut context = prepare_contract(None, [product]).await?;

    let alice = context.alice().await?;

    for _ in 0..jars_count {
        add_jar(&context, &alice, product, 100_000).await?;
    }

    context.fast_forward_hours(2).await?;

    Ok(context
        .sweat_jar()
        .claim_total(None)
        .with_user(&alice)
        .result()
        .await?
        .total_gas_burnt)
}
