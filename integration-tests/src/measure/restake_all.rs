use anyhow::Result;
use nitka::{measure::utils::pretty_gas_string, set_integration_logs_enabled};
use sweat_jar_model::api::{ClaimApiIntegration, JarApiIntegration};

use crate::{
    context::{prepare_contract, IntegrationContext},
    measure::utils::add_jar,
    product::RegisterProductCommand,
};

#[ignore]
#[tokio::test]
#[mutants::skip]
async fn measure_restake_all() -> Result<()> {
    set_integration_logs_enabled(false);

    let product = RegisterProductCommand::Locked5Minutes60000Percents;
    let mut context = prepare_contract(None, [product]).await?;
    let alice = context.alice().await?;

    for _ in 0..200 {
        add_jar(&context, &alice, product, 10_000).await?;
    }

    context.fast_forward_minutes(6).await?;

    context.sweat_jar().claim_total(None).with_user(&alice).await?;

    let gas = context
        .sweat_jar()
        .restake_all()
        .with_user(&alice)
        .result()
        .await?
        .total_gas_burnt;
    dbg!(pretty_gas_string(gas));

    //   1  jar -  6 TGas 225 GGas total:  6225437862976
    // 100 jars - 50 TGas 709 GGas total: 50709431315947
    // 200 jars - 86 TGas 607 GGas total: 86607517267105

    Ok(())
}
