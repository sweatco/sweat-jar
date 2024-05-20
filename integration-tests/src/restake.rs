use anyhow::Result;
use nitka::{misc::ToNear, set_integration_logs_enabled};
use sweat_jar_model::api::{ClaimApiIntegration, JarApiIntegration};

use crate::{
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
    let original_jar_id = jars.first().unwrap().id;

    context.fast_forward_hours(1).await?;

    context.sweat_jar().restake(original_jar_id).with_user(&alice).await?;

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(jars.len(), 2);

    let mut has_original_jar = false;
    let mut has_restaked_jar = false;
    for jar in jars {
        let id = jar.id;

        if id == original_jar_id {
            has_original_jar = true;
            assert_eq!(jar.principal.0, 0);
        } else {
            has_restaked_jar = true;
            assert_eq!(jar.principal.0, amount);
        }
    }

    assert!(has_original_jar);
    assert!(has_restaked_jar);

    context
        .sweat_jar()
        .claim_jars(vec![original_jar_id], None, None)
        .with_user(&alice)
        .await?;

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;
    assert_eq!(jars.len(), 1);

    Ok(())
}

#[tokio::test]
#[mutants::skip]
async fn restake_all() -> Result<()> {
    const PRINCIPAL: u128 = 1_000_000;
    const JARS_COUNT: u16 = 210;

    println!("👷🏽 Run test for restake all");

    set_integration_logs_enabled(false);

    let product_5_min = RegisterProductCommand::Locked5Minutes60000Percents;
    let product_10_min = RegisterProductCommand::Locked10Minutes60000Percents;

    let mut context = prepare_contract(None, [product_5_min, product_10_min]).await?;

    let alice = context.alice().await?;

    let amount = context
        .sweat_jar()
        .create_jar(&alice, product_5_min.id(), PRINCIPAL + 1, &context.ft_contract())
        .await?;
    assert_eq!(amount.0, PRINCIPAL + 1);

    let jar_5_min_1 = context.last_jar_for(&alice).await?;
    assert_eq!(jar_5_min_1.principal.0, PRINCIPAL + 1);

    context
        .sweat_jar()
        .create_jar(&alice, product_5_min.id(), PRINCIPAL + 2, &context.ft_contract())
        .await?;
    let jar_5_min_2 = context.last_jar_for(&alice).await?;
    assert_eq!(jar_5_min_2.principal.0, PRINCIPAL + 2);

    context
        .sweat_jar()
        .create_jar(&alice, product_10_min.id(), PRINCIPAL + 3, &context.ft_contract())
        .await?;
    let jar_10_min = context.last_jar_for(&alice).await?;
    assert_eq!(jar_10_min.principal.0, PRINCIPAL + 3);

    context
        .bulk_create_jars(&alice, &product_5_min.id(), PRINCIPAL, JARS_COUNT)
        .await?;

    let claimed = context.sweat_jar().claim_total(None).await?;
    assert_eq!(claimed.get_total().0, 0);

    context.fast_forward_minutes(6).await?;

    context.sweat_jar().claim_total(None).with_user(&alice).await?;

    let restaked = context.sweat_jar().restake_all().with_user(&alice).await?; // 212 jars: ⛽ 91 TGas 566 GGas total: 91566686658202. 1 jar: ⛽ 6 TGas 410 GGas total: 6410903482276

    assert_eq!(restaked.len(), 212);

    assert_eq!(
        restaked.into_iter().map(|j| j.principal).collect::<Vec<_>>()[..2],
        vec![jar_5_min_1.principal, jar_5_min_2.principal]
    );

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;

    let principals = jars.iter().map(|j| j.principal.0).collect::<Vec<_>>();

    assert!(
        [PRINCIPAL + 3, PRINCIPAL + 1, PRINCIPAL + 2]
            .iter()
            .all(|p| principals.contains(p)),
        "Can't find all expected principals in {principals:?}"
    );

    Ok(())
}
