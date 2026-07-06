use anyhow::Result;
use sweat_jar_model::{
    data::{
        deposit::{DepositMessage, Purpose},
        product::Product,
    },
    signer::test_utils::MessageSigner,
};
use tracing::info;

mod common;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand::Locked10Minutes20000ScoreCap};

#[tokio::test]
#[tracing::instrument]
async fn record_score_dos() -> Result<()> {
    common::prepare::init_tracing();
    info!("record score DOS test");

    let signer = MessageSigner::new();
    let product = Product {
        public_key: Some(signer.public_key().into()),
        ..Locked10Minutes20000ScoreCap.get()
    };

    let context = prepare_contract([]).await?;

    jar::register_product(&context.jar, &context.manager, product.clone()).await?;

    let deposit_amount = 100000;
    let valid_until = 49_012_505_000_000;
    let deposit_message = DepositMessage::new(
        Purpose::Deposit,
        context.jar.id(),
        context.alice.id(),
        &product.id,
        deposit_amount,
        valid_until,
        0,
    );
    jar::create_step_jar(
        &context.jar,
        &context.ft,
        &context.alice,
        &Locked10Minutes20000ScoreCap.id(),
        deposit_amount,
        signer.sign(deposit_message.as_str()).into(),
        valid_until,
        0,
    )
    .await?;

    context
        .bulk_create_jars(&context.alice, &Locked10Minutes20000ScoreCap.id(), 100000, 1400)
        .await?;

    let now = jar::block_timestamp_ms(&context.jar).await?;

    // Raw call (not the `jar::record_score` helper) because this test needs the
    // raw `ExecutionFinalResult` to inspect `.logs()` for the emitted event.
    let record_result = context
        .manager
        .call(context.jar.id(), "record_score")
        .args_json(serde_json::json!({ "batch": [(context.alice.id(), [(5000u16, now)])] }))
        .max_gas()
        .transact()
        .await?;

    assert!(record_result
        .logs()
        .first()
        .unwrap()
        .contains(r#""event": "record_score""#));

    Ok(())
}
