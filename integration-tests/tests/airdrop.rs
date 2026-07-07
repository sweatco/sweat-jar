use anyhow::Result;
use serde_json::Value;
use sweat_jar_model::{
    data::{deposit::AirdropMessage, product::Product},
    signer::test_utils::MessageSigner,
};
use tracing::info;

mod common;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
#[tracing::instrument]
async fn airdrop_basic() -> Result<()> {
    common::prepare::init_tracing();
    info!("airdrop basic test");

    let context = prepare_contract([RegisterProductCommand::Locked10Minutes6Percents]).await?;

    let amount_per_receiver = 1_000_000u128;

    jar::airdrop(
        &context.jar,
        &context.ft,
        &context.manager,
        &RegisterProductCommand::Locked10Minutes6Percents.id(),
        amount_per_receiver,
        &[context.alice.clone(), context.bob.clone()],
        None,
    )
    .await?;

    let alice_jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(amount_per_receiver, alice_jars.get_total_principal());

    let bob_jars = jar::get_jars_for_account(&context.jar, context.bob.id()).await?;
    assert_eq!(amount_per_receiver, bob_jars.get_total_principal());

    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn airdrop_score_based_sets_timezone() -> Result<()> {
    common::prepare::init_tracing();
    info!("airdrop score-based timezone test");

    let signer = MessageSigner::new();
    let product = Product {
        public_key: Some(signer.public_key().into()),
        ..RegisterProductCommand::Locked10Minutes20000ScoreCap.get()
    };

    let context = prepare_contract([]).await?;

    jar::register_product(&context.jar, &context.manager, product.clone()).await?;

    let amount_per_receiver = 1_000_000u128;
    let timezone = 3;
    let valid_until = 49_012_505_000_000u64;
    let receivers = [context.alice.clone(), context.bob.clone()];

    let airdrop_message = AirdropMessage::new(
        context.jar.id(),
        &product.id,
        amount_per_receiver,
        &[context.alice.id().clone(), context.bob.id().clone()],
        valid_until,
    );
    let signature = signer.sign(airdrop_message.material());

    jar::airdrop_protected(
        &context.jar,
        &context.ft,
        &context.manager,
        &product.id,
        amount_per_receiver,
        &receivers,
        Some(timezone),
        signature.into(),
        valid_until,
        0,
        None,
    )
    .await?;

    let alice_jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(amount_per_receiver, alice_jars.get_total_principal());

    let bob_jars = jar::get_jars_for_account(&context.jar, context.bob.id()).await?;
    assert_eq!(amount_per_receiver, bob_jars.get_total_principal());

    let airdrop_message2 = AirdropMessage::new(
        context.jar.id(),
        &product.id,
        amount_per_receiver,
        &[context.alice.id().clone()],
        valid_until,
    );
    let signature2 = signer.sign(airdrop_message2.material());

    jar::airdrop_protected(
        &context.jar,
        &context.ft,
        &context.manager,
        &product.id,
        amount_per_receiver,
        &[context.alice.clone()],
        Some(0),
        signature2.into(),
        valid_until,
        0,
        None,
    )
    .await?;

    let alice_jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(amount_per_receiver * 2, alice_jars.get_total_principal());

    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn airdrop_by_non_oracle_fails() -> Result<()> {
    common::prepare::init_tracing();
    info!("airdrop by non-oracle test");

    let context = prepare_contract([RegisterProductCommand::Locked10Minutes6Percents]).await?;

    jar::airdrop(
        &context.jar,
        &context.ft,
        &context.alice,
        &RegisterProductCommand::Locked10Minutes6Percents.id(),
        1_000_000,
        &[context.bob.clone()],
        None,
    )
    .await?;

    let bob_jars = jar::get_jars_for_account(&context.jar, context.bob.id()).await?;
    assert_eq!(0, bob_jars.get_total_principal());

    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn airdrop_with_booster() -> Result<()> {
    common::prepare::init_tracing();
    info!("airdrop with booster test");

    let signer = MessageSigner::new();
    let product = Product {
        public_key: Some(signer.public_key().into()),
        ..RegisterProductCommand::Locked10Minutes20000ScoreCap.get()
    };

    let context = prepare_contract([]).await?;

    jar::register_product(&context.jar, &context.manager, product.clone()).await?;

    let amount_per_receiver = 1_000_000u128;
    let timezone = 0;
    let valid_until = 49_012_505_000_000u64;
    let booster = 5_000u16;

    let receivers = [context.alice.clone()];
    let airdrop_message = AirdropMessage::new(
        context.jar.id(),
        &product.id,
        amount_per_receiver,
        &[context.alice.id().clone()],
        valid_until,
    );
    let signature = signer.sign(airdrop_message.material());

    let result = jar::airdrop_protected_raw(
        &context.jar,
        &context.ft,
        &context.manager,
        &product.id,
        amount_per_receiver,
        &receivers,
        Some(timezone),
        signature.into(),
        valid_until,
        booster,
        None,
    )
    .await?
    .into_result()?;

    let alice_jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(amount_per_receiver, alice_jars.get_total_principal());

    // Principal alone doesn't prove the booster was actually applied — booster
    // and principal are tracked independently, and `get_score`'s view only
    // reflects recorded score, never the booster field. The `ApplyBooster`
    // event is the only observable proof the booster reached alice's account.
    let apply_booster_event = result
        .logs()
        .iter()
        .find_map(|log| {
            let json = log.strip_prefix("EVENT_JSON:")?;
            let event: Value = serde_json::from_str(json).ok()?;
            (event["event"] == "apply_booster").then_some(event)
        })
        .expect("an ApplyBooster event should have been emitted");

    let applied = apply_booster_event["data"]["applied"]
        .as_array()
        .expect("ApplyBooster event should have an 'applied' array");
    assert_eq!(
        1,
        applied.len(),
        "exactly one account should have the booster applied: {applied:?}"
    );
    assert_eq!(context.alice.id().as_str(), applied[0].as_str().unwrap());

    let rejected = apply_booster_event["data"]["rejected"]
        .as_array()
        .expect("ApplyBooster event should have a 'rejected' array");
    assert!(rejected.is_empty(), "no account should have been rejected: {rejected:?}");

    Ok(())
}
