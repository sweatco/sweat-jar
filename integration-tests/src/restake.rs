use std::collections::HashSet;

use anyhow::Result;
use nitka::{misc::ToNear, set_integration_logs_enabled};
use sweat_jar_model::{api::*, data::deposit::DepositTicket, TokenAmount};

use crate::{
    common::total_principal,
    context::{prepare_contract, ContextHelpers, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn restake() -> Result<()> {
    println!("👷🏽 Run test for restaking");

    let product = RegisterProductCommand::Locked10Minutes6Percents;

    let mut context = prepare_contract(None, [product]).await?;

    let alice = context.alice().await?;

    let amount = 1_000_000;
    context
        .sweat_jar()
        .create_jar(&alice, product.id(), amount, &context.ft_contract())
        .await?;

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(1, jars.len());
    assert_eq!(amount, total_principal(&jars));

    let first_jar_timestamp = jars.first().unwrap().created_at.0;

    context.fast_forward_hours(1).await?;
    let ticket = DepositTicket {
        product_id: product.get().id,
        valid_until: 0.into(),
        timezone: None,
    };
    context
        .sweat_jar()
        .restake(product.get().id, ticket, None, None)
        .with_user(&alice)
        .await?;

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(1, jars.len());
    assert_eq!(amount, total_principal(&jars));

    let second_jar_timestamp = jars.first().unwrap().created_at.0;
    assert!(second_jar_timestamp > first_jar_timestamp);

    context.sweat_jar().claim_total(None).with_user(&alice).await?;

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(jars.len(), 1);

    Ok(())
}

#[tokio::test]
#[mutants::skip]
async fn restake_all() -> Result<()> {
    const PRINCIPAL: u128 = 1_000_000;
    const JARS_COUNT: u16 = 5010;

    println!("👷🏽 Run test for restake all");

    set_integration_logs_enabled(false);

    let product_5_min = RegisterProductCommand::Locked5Minutes60000Percents;
    let product_10_min = RegisterProductCommand::Locked10Minutes60000Percents;

    let mut product_5_min_total = 0;
    let mut product_10_min_total = 0;

    let mut context = prepare_contract(None, [product_5_min, product_10_min]).await?;

    let alice = context.alice().await?;

    product_5_min_total += PRINCIPAL + 1;
    let amount = context
        .sweat_jar()
        .create_jar(&alice, product_5_min.id(), PRINCIPAL + 1, &context.ft_contract())
        .await?;
    assert_eq!(amount.0, PRINCIPAL + 1);

    let jar_5_min_1 = context.last_jar_for(&alice).await?;
    assert_eq!(jar_5_min_1.principal.0, PRINCIPAL + 1);

    product_5_min_total += PRINCIPAL + 2;
    context
        .sweat_jar()
        .create_jar(&alice, product_5_min.id(), PRINCIPAL + 2, &context.ft_contract())
        .await?;
    let jar_5_min_2 = context.last_jar_for(&alice).await?;
    assert_eq!(jar_5_min_2.principal.0, PRINCIPAL + 2);

    product_10_min_total += PRINCIPAL + 3;
    context
        .sweat_jar()
        .create_jar(&alice, product_10_min.id(), PRINCIPAL + 3, &context.ft_contract())
        .await?;
    let jar_10_min = context.last_jar_for(&alice).await?;
    assert_eq!(jar_10_min.principal.0, PRINCIPAL + 3);

    product_5_min_total += JARS_COUNT as u128 * PRINCIPAL;
    context
        .bulk_create_jars(&alice, &product_5_min.id(), PRINCIPAL, JARS_COUNT)
        .await?;

    let claimed = context.sweat_jar().claim_total(None).await;
    assert!(claimed.is_err());

    context.fast_forward_minutes(6).await?;

    context.sweat_jar().claim_total(None).with_user(&alice).await?;

    // Restaking in batches
    let ticket = DepositTicket {
        product_id: product_5_min.id(),
        valid_until: 0.into(),
        timezone: None,
    };
    context
        .sweat_jar()
        .restake_all(ticket, None, None)
        .with_user(&alice)
        .await?;

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    let principals_set: HashSet<TokenAmount> = HashSet::from_iter(jars.iter().map(|j| j.principal.0));

    assert_eq!(
        HashSet::from_iter([product_5_min_total, product_10_min_total]),
        principals_set
    );

    Ok(())
}
