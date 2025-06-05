use std::collections::HashMap;

use anyhow::Result;
use near_workspaces::types::Gas;
use nitka::set_integration_logs_enabled;

use crate::{
    context::{prepare_contract, prepare_contract_with_legacy, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    measure::{
        measure::scoped_command_measure,
        utils::{add_jar, append_measure, generate_permutations, measure_jars_range, retry_until_ok, MeasureData},
    },
    product::{RegisterProductCommand, RegisterProductCommand::Locked10Minutes6Percents},
};

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn measure_stake_total_test() -> Result<()> {
    use RegisterProductCommand::*;

    async fn stake() -> Result<()> {
        let measured = scoped_command_measure(
            generate_permutations(
                &[
                    Locked10Minutes6Percents,
                    Locked12Months12Percents,
                    Locked6Months6Percents,
                    Flexible6Months6Percents,
                    Locked6Months6PercentsWithWithdrawFee,
                ],
                &measure_jars_range(),
            ),
            measure_stake,
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

        append_measure("stake", map)
    }

    retry_until_ok(stake).await?;

    Ok(())
}

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn one_stake() -> anyhow::Result<()> {
    let gas = measure_stake((RegisterProductCommand::Locked10Minutes6Percents, 1)).await?;

    dbg!(&gas);

    Ok(())
}

#[mutants::skip]
pub(crate) async fn measure_stake(input: (RegisterProductCommand, usize)) -> anyhow::Result<Gas> {
    let (product, jars_count) = input;

    let mut context = prepare_contract(None, [product]).await?;

    let alice = context.alice().await?;

    for _ in 0..jars_count {
        add_jar(&context, &alice, product, 100_000).await?;
    }

    Ok(context
        .sweat_jar()
        .create_jar(&alice, product.id(), 100_000, &context.ft_contract())
        .result()
        .await?
        .total_gas_burnt)
}

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn measure_storage_on_staking() -> Result<()> {
    const DEPOSIT: u128 = 2 * 10u128.pow(18);

    set_integration_logs_enabled(false);

    let product = Locked10Minutes6Percents;
    let mut context = prepare_contract_with_legacy(None, [product]).await?;
    let alice = context.alice().await?;

    let jars_account = context.sweat_jar().contract.as_account();
    let jars_initial_storage_usage = jars_account.view_account().await?.storage_usage;
    dbg!(&jars_initial_storage_usage);

    let legacy_jars_account = context.sweat_jar_legacy().contract.as_account();
    let legacy_jars_initial_storage_usage = legacy_jars_account.view_account().await?.storage_usage;
    dbg!(&legacy_jars_initial_storage_usage);

    let mut csv = "count,v1,v2".to_owned();
    for i in 0..400 {
        let result = context
            .sweat_jar()
            .create_jar(&alice, product.id(), DEPOSIT, &context.ft_contract())
            .await?;
        assert_eq!(DEPOSIT, result.0);

        let result = context
            .sweat_jar_legacy()
            .create_jar(&alice, product.id(), DEPOSIT, &context.ft_contract())
            .await?;
        assert_eq!(DEPOSIT, result.0);

        let v1_storage_usage = legacy_jars_account.view_account().await?.storage_usage;
        let v2_storage_usage = jars_account.view_account().await?.storage_usage;

        csv.push_str(format!("\n{i},{v1_storage_usage},{v2_storage_usage}").as_str());
    }

    print!("{}", csv);

    Ok(())
}
