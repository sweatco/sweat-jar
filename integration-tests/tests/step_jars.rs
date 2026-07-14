mod common;

use anyhow::Result;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand::Locked10Minutes20000ScoreCap};
use sweat_jar_model::Timezone;

#[tokio::test]
async fn record_score_dos() -> Result<()> {
    let context = prepare_contract([Locked10Minutes20000ScoreCap]).await?;
    let alice = &context.alice;
    let manager = &context.manager;

    jar::create_step_jar(
        &context.ft,
        &context.jar,
        alice,
        &Locked10Minutes20000ScoreCap.id(),
        100_000,
        Timezone::hour_shift(0),
    )
    .await?
    .into_result()?;

    jar::bulk_create_jars(
        &context.jar,
        manager,
        alice.id(),
        &Locked10Minutes20000ScoreCap.id(),
        100_000,
        1400,
    )
    .await?;

    let now = jar::block_timestamp_ms(&context.jar).await?;

    //    1  jar - ⛽   6 TGas 273 GGas total:   6273920462025
    // 1401 jars - ⛽ 270 TGas 476 GGas total: 270476838486762
    let result = jar::record_score(&context.jar, manager, vec![(alice.id(), vec![(5000, now.into())])])
        .await?
        .into_result()?;

    assert!(result
        .logs()
        .iter()
        .any(|log| log.contains(r#""event": "record_score""#)));

    Ok(())
}
