use std::collections::HashSet;

use anyhow::Result;
use sweat_jar_model::{data::deposit::DepositTicket, TokenAmount};
use tracing::info;

mod common;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
#[tracing::instrument]
async fn restake() -> Result<()> {
    common::prepare::init_tracing();
    info!("restake test");

    let product = RegisterProductCommand::Locked10Minutes6Percents;
    let context = prepare_contract([product]).await?;

    let amount = 1_000_000;
    jar::create_jar(&context.jar, &context.ft, &context.alice, &product.id(), amount).await?;

    let jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(1, jars.get_total_deposits_number());
    assert_eq!(amount, jars.get_total_principal());

    let first_jar_timestamp = jars.0.get(&product.id()).unwrap().first().unwrap().0;

    context.fast_forward_hours(1).await?;
    let ticket = DepositTicket {
        product_id: product.get().id,
        valid_until: 0.into(),
        timezone: None,
    };
    jar::restake(&context.jar, &context.alice, &product.get().id, ticket, None, None).await?;

    let jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(1, jars.get_total_deposits_number());
    assert_eq!(amount, jars.get_total_principal());

    let second_jar_timestamp = jars.0.get(&product.id()).unwrap().first().unwrap().0;
    assert!(second_jar_timestamp > first_jar_timestamp);

    jar::claim_total(&context.jar, &context.alice, None).await?;

    let jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    assert_eq!(jars.get_total_deposits_number(), 1);

    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn restake_all() -> Result<()> {
    const PRINCIPAL: u128 = 1_000_000;
    const JARS_COUNT: u16 = 5010;

    common::prepare::init_tracing();
    info!("restake all test");

    let product_5_min = RegisterProductCommand::Locked5Minutes60000Percents;
    let product_10_min = RegisterProductCommand::Locked10Minutes60000Percents;

    let mut product_5_min_total = 0;
    let mut product_10_min_total = 0;

    let context = prepare_contract([product_5_min, product_10_min]).await?;

    product_5_min_total += PRINCIPAL + 1;
    let amount = jar::create_jar(&context.jar, &context.ft, &context.alice, &product_5_min.id(), PRINCIPAL + 1).await?;
    assert_eq!(amount, PRINCIPAL + 1);

    product_5_min_total += PRINCIPAL + 2;
    jar::create_jar(&context.jar, &context.ft, &context.alice, &product_5_min.id(), PRINCIPAL + 2).await?;

    product_10_min_total += PRINCIPAL + 3;
    jar::create_jar(&context.jar, &context.ft, &context.alice, &product_10_min.id(), PRINCIPAL + 3).await?;

    product_5_min_total += JARS_COUNT as u128 * PRINCIPAL;
    context
        .bulk_create_jars(&context.alice, &product_5_min.id(), PRINCIPAL, JARS_COUNT)
        .await?;

    context.fast_forward_minutes(6).await?;

    jar::claim_total(&context.jar, &context.alice, None).await?;

    let ticket = DepositTicket {
        product_id: product_5_min.id(),
        valid_until: 0.into(),
        timezone: None,
    };
    jar::restake_all(&context.jar, &context.alice, ticket, None, None).await?;

    let jars = jar::get_jars_for_account(&context.jar, context.alice.id()).await?;
    let principals_set: HashSet<TokenAmount> = jars
        .0
        .values()
        .flat_map(|deposits| deposits.iter().map(|(_, principal)| principal.0))
        .collect();

    assert_eq!(
        HashSet::from_iter([product_5_min_total, product_10_min_total]),
        principals_set
    );

    Ok(())
}
