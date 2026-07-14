mod common;

use anyhow::Result;
use common::{jar, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
async fn restake() -> Result<()> {
    let product = RegisterProductCommand::Locked10Minutes6Percents;

    let context = prepare_contract([product]).await?;
    let alice = &context.alice;

    let amount = 1_000_000;
    jar::create_jar(&context.ft, &context.jar, alice, &product.id(), amount)
        .await?
        .into_result()?;

    let jars = jar::get_jars_for_account(&context.jar, alice.id()).await?;
    let original_jar_id = jars.first().unwrap().id;

    context.fast_forward_hours(1).await?;

    jar::restake(&context.jar, alice, original_jar_id).await?;

    let jars = jar::get_jars_for_account(&context.jar, alice.id()).await?;
    assert_eq!(jars.len(), 2);

    let mut has_original_jar = false;
    let mut has_restaked_jar = false;
    for jar_view in jars {
        if jar_view.id == original_jar_id {
            has_original_jar = true;
            assert_eq!(jar_view.principal.0, 0);
        } else {
            has_restaked_jar = true;
            assert_eq!(jar_view.principal.0, amount);
        }
    }

    assert!(has_original_jar);
    assert!(has_restaked_jar);

    jar::claim_total(&context.jar, alice, None).await?;

    let jars = jar::get_jars_for_account(&context.jar, alice.id()).await?;
    assert_eq!(jars.len(), 1);

    Ok(())
}

#[tokio::test]
async fn restake_all() -> Result<()> {
    const PRINCIPAL: u128 = 1_000_000;
    const JARS_COUNT: u16 = 210;

    let product_5_min = RegisterProductCommand::Locked5Minutes60000Percents;
    let product_10_min = RegisterProductCommand::Locked10Minutes60000Percents;

    let context = prepare_contract([product_5_min, product_10_min]).await?;
    let alice = &context.alice;

    let amount =
        jar::used_amount(jar::create_jar(&context.ft, &context.jar, alice, &product_5_min.id(), PRINCIPAL + 1).await?)?;
    assert_eq!(amount, PRINCIPAL + 1);

    let jar_5_min_1 = context.last_jar_for(alice).await?;
    assert_eq!(jar_5_min_1.principal.0, PRINCIPAL + 1);

    jar::create_jar(&context.ft, &context.jar, alice, &product_5_min.id(), PRINCIPAL + 2)
        .await?
        .into_result()?;
    let jar_5_min_2 = context.last_jar_for(alice).await?;
    assert_eq!(jar_5_min_2.principal.0, PRINCIPAL + 2);

    jar::create_jar(&context.ft, &context.jar, alice, &product_10_min.id(), PRINCIPAL + 3)
        .await?
        .into_result()?;
    let jar_10_min = context.last_jar_for(alice).await?;
    assert_eq!(jar_10_min.principal.0, PRINCIPAL + 3);

    context
        .bulk_create_jars(alice, &product_5_min.id(), PRINCIPAL, JARS_COUNT)
        .await?;

    context.fast_forward_minutes(6).await?;

    jar::claim_total(&context.jar, alice, None).await?;

    // Restaking happens in batches
    let restaked = jar::restake_all(&context.jar, alice).await?;
    assert_eq!(restaked.len(), 200);

    let restaked_2 = jar::restake_all(&context.jar, alice).await?;
    assert_eq!(restaked_2.len(), 12);

    assert_eq!(
        restaked.into_iter().map(|j| j.principal).collect::<Vec<_>>()[..2],
        vec![jar_5_min_1.principal, jar_5_min_2.principal]
    );

    let jars = jar::get_jars_for_account(&context.jar, alice.id()).await?;
    let principals = jars.iter().map(|j| j.principal.0).collect::<Vec<_>>();

    assert!(
        [PRINCIPAL + 3, PRINCIPAL + 1, PRINCIPAL + 2]
            .iter()
            .all(|p| principals.contains(p)),
        "Can't find all expected principals in {principals:?}"
    );

    Ok(())
}
