use std::collections::HashMap;

use anyhow::Result;
use near_workspaces::types::Gas;
use nitka::misc::ToNear;
use sweat_jar_model::api::JarApiIntegration;

use crate::{
    context::{prepare_contract, IntegrationContext},
    measure::{
        measure::scoped_command_measure,
        random_element::RandomElement,
        utils::{add_jar, append_measure, generate_permutations, measure_jars_range, retry_until_ok, MeasureData},
    },
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn measure_restake_total_test() -> Result<()> {
    async fn restake() -> Result<()> {
        let measured = scoped_command_measure(
            generate_permutations(
                &[RegisterProductCommand::Locked10Minutes6Percents],
                &measure_jars_range(),
            ),
            measure_restake,
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

        append_measure("restake", map)
    }

    retry_until_ok(restake).await?;

    Ok(())
}

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn one_restake() -> anyhow::Result<()> {
    let gas = measure_restake((RegisterProductCommand::Locked10Minutes6Percents, 1)).await?;

    dbg!(&gas);

    Ok(())
}

#[mutants::skip]
pub(crate) async fn measure_restake(input: (RegisterProductCommand, usize)) -> anyhow::Result<Gas> {
    let (product, jars_count) = input;

    let mut context = prepare_contract(None, [product]).await?;

    let alice = context.alice().await?;

    for _ in 0..jars_count {
        add_jar(&context, &alice, product, 100_000).await?;
    }

    context.fast_forward_hours(2).await?;

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;

    Ok(context
        .sweat_jar()
        .restake(jars.random_element().id)
        .with_user(&alice)
        .result()
        .await?
        .total_gas_burnt)
}
