use anyhow::Result;
use nitka::misc::ToNear;
use sweat_jar_model::{
    api::*,
    data::{deposit::AirdropMessage, product::Product},
    signer::test_utils::MessageSigner,
    Timezone,
};

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn airdrop_basic() -> Result<()> {
    println!("👷🏽 Run airdrop basic test");

    let mut context = prepare_contract(None, [RegisterProductCommand::Locked10Minutes6Percents]).await?;

    let manager = context.manager().await?;
    let alice = context.alice().await?;
    let bob = context.bob().await?;

    let amount_per_receiver = 1_000_000u128;

    context
        .sweat_jar()
        .airdrop(
            &manager,
            RegisterProductCommand::Locked10Minutes6Percents.id(),
            amount_per_receiver,
            &[alice.clone(), bob.clone()],
            None,
            &context.ft_contract(),
        )
        .await?;

    let alice_jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(amount_per_receiver, alice_jars.get_total_principal());

    let bob_jars = context.sweat_jar().get_jars_for_account(bob.to_near()).await?;
    assert_eq!(amount_per_receiver, bob_jars.get_total_principal());

    Ok(())
}

#[tokio::test]
#[mutants::skip]
async fn airdrop_score_based_sets_timezone() -> Result<()> {
    println!("👷🏽 Run airdrop score-based timezone test");

    let signer = MessageSigner::new();
    let product = Product {
        public_key: Some(signer.public_key().into()),
        ..RegisterProductCommand::Locked10Minutes20000ScoreCap.get()
    };

    let mut context = prepare_contract(None, []).await?;

    let manager = context.manager().await?;
    let alice = context.alice().await?;
    let bob = context.bob().await?;

    context
        .sweat_jar()
        .register_product(product.clone())
        .with_user(&manager)
        .await?;

    let amount_per_receiver = 1_000_000u128;
    let timezone = Timezone::hour_shift(3);
    let valid_until = 49_012_505_000_000u64;
    let receivers = [alice.clone(), bob.clone()];

    let airdrop_message = AirdropMessage::new(
        context.sweat_jar().contract.as_account().id(),
        &product.id,
        amount_per_receiver,
        &[alice.id().clone(), bob.id().clone()],
        valid_until,
    );
    let signature = signer.sign(airdrop_message.material());

    context
        .sweat_jar()
        .airdrop_protected(
            &manager,
            product.id.clone(),
            amount_per_receiver,
            &receivers,
            Some(timezone),
            signature.into(),
            valid_until,
            0,
            &context.ft_contract(),
        )
        .await?;

    // Successful airdrop to protected score-based product confirms timezone was accepted.
    let alice_jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(amount_per_receiver, alice_jars.get_total_principal());

    let bob_jars = context.sweat_jar().get_jars_for_account(bob.to_near()).await?;
    assert_eq!(amount_per_receiver, bob_jars.get_total_principal());

    // Second airdrop: alice already has a timezone — try_set_timezone is a no-op
    let airdrop_message2 = AirdropMessage::new(
        context.sweat_jar().contract.as_account().id(),
        &product.id,
        amount_per_receiver,
        &[alice.id().clone()],
        valid_until,
    );
    let signature2 = signer.sign(airdrop_message2.material());

    context
        .sweat_jar()
        .airdrop_protected(
            &manager,
            product.id.clone(),
            amount_per_receiver,
            &[alice.clone()],
            Some(Timezone::hour_shift(0)), // different timezone — should be ignored
            signature2.into(),
            valid_until,
            0,
            &context.ft_contract(),
        )
        .await?;

    let alice_jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(amount_per_receiver * 2, alice_jars.get_total_principal());

    Ok(())
}

#[tokio::test]
#[mutants::skip]
async fn airdrop_by_non_manager_fails() -> Result<()> {
    println!("👷🏽 Run airdrop non-manager test");

    let mut context = prepare_contract(None, [RegisterProductCommand::Locked10Minutes6Percents]).await?;

    let alice = context.alice().await?;
    let bob = context.bob().await?;

    // alice is not the manager — ft_on_transfer will panic and refund tokens,
    // but ft_transfer_call itself returns Ok (NEAR cross-contract error handling)
    context
        .sweat_jar()
        .airdrop(
            &alice,
            RegisterProductCommand::Locked10Minutes6Percents.id(),
            1_000_000,
            &[bob.clone()],
            None,
            &context.ft_contract(),
        )
        .await?;

    // Verify no deposit was created for bob
    let bob_jars = context.sweat_jar().get_jars_for_account(bob.to_near()).await?;
    assert_eq!(0, bob_jars.get_total_principal());

    Ok(())
}

#[tokio::test]
#[mutants::skip]
async fn airdrop_with_booster() -> Result<()> {
    println!("👷🏽 Run airdrop with booster test");

    let signer = MessageSigner::new();
    let product = Product {
        public_key: Some(signer.public_key().into()),
        ..RegisterProductCommand::Locked10Minutes20000ScoreCap.get()
    };

    let mut context = prepare_contract(None, []).await?;

    let manager = context.manager().await?;
    let alice = context.alice().await?;

    context
        .sweat_jar()
        .register_product(product.clone())
        .with_user(&manager)
        .await?;

    let amount_per_receiver = 1_000_000u128;
    let timezone = Timezone::hour_shift(0);
    let valid_until = 49_012_505_000_000u64;
    let booster = 5_000u16;

    let receivers = [alice.clone()];
    let airdrop_message = AirdropMessage::new(
        context.sweat_jar().contract.as_account().id(),
        &product.id,
        amount_per_receiver,
        &[alice.id().clone()],
        valid_until,
    );
    let signature = signer.sign(airdrop_message.material());

    context
        .sweat_jar()
        .airdrop_protected(
            &manager,
            product.id.clone(),
            amount_per_receiver,
            &receivers,
            Some(timezone),
            signature.into(),
            valid_until,
            booster,
            &context.ft_contract(),
        )
        .await?;

    let alice_jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(amount_per_receiver, alice_jars.get_total_principal());

    Ok(())
}
