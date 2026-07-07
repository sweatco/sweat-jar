use anyhow::Result;
use sweat_jar_model::{
    data::{deposit::AirdropMessage, product::Product},
    signer::test_utils::MessageSigner,
    MS_IN_DAY,
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
    // Signed tickets must satisfy the on-chain upper bound on `valid_until`
    // (at most 7 real days from now), so anchor it to the chain's clock.
    let valid_until = jar::block_timestamp_ms(&context.jar).await? + MS_IN_DAY;
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
    // Signed tickets must satisfy the on-chain upper bound on `valid_until`
    // (at most 7 real days from now), so anchor it to the chain's clock.
    let valid_until = jar::block_timestamp_ms(&context.jar).await? + MS_IN_DAY;
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
        booster,
        None,
    )
    .await?;

    let alice_jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(amount_per_receiver, alice_jars.get_total_principal());

    Ok(())
}
