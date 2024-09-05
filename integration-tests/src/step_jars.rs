use anyhow::Result;
use nitka::{misc::ToNear, set_integration_logs_enabled};
use sweat_jar_model::{
    api::{IntegrationTestMethodsIntegration, ScoreApiIntegration},
    Timezone,
};

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
    step_jars::RegisterProductCommand::Locked10Minutes20000ScoreCap,
};

#[tokio::test]
#[mutants::skip]
async fn record_score_dos() -> Result<()> {
    println!("👷🏽 Run record score DOS test");

    let product = RegisterProductCommand::Locked10Minutes20000ScoreCap;

    set_integration_logs_enabled(false);

    let mut context = prepare_contract(None, [product]).await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;

    context
        .sweat_jar()
        .create_step_jar(
            &alice,
            Locked10Minutes20000ScoreCap.id(),
            100000,
            Timezone::hour_shift(0),
            &context.ft_contract(),
        )
        .await?;

    context
        .sweat_jar()
        .bulk_create_jars(alice.to_near(), Locked10Minutes20000ScoreCap.id(), 100000, 1400)
        .with_user(&manager)
        .await?;

    let now = context.sweat_jar().block_timestamp_ms().await?;

    set_integration_logs_enabled(true);

    //    1  jar - ⛽   6 TGas 273 GGas total:   6273920462025
    // 1401 jars - ⛽ 270 TGas 476 GGas total: 270476838486762
    let result = context
        .sweat_jar()
        .record_score(vec![(alice.to_near(), vec![(5000, now.into())])])
        .with_user(&manager)
        .result()
        .await?;

    assert!(result.logs().first().unwrap().contains(r#""event": "record_score""#));

    Ok(())
}
