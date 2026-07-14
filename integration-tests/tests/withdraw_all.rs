mod common;

use anyhow::Result;
use common::{ft, jar, prepare::prepare_contract, product::RegisterProductCommand};

#[tokio::test]
async fn withdraw_all() -> Result<()> {
    const PRINCIPAL: u128 = 1_000_000;
    const JARS_COUNT: u16 = 210;
    const BULK_PRINCIPAL: u128 = PRINCIPAL * JARS_COUNT as u128;

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

    context
        .bulk_create_jars(alice, &product_5_min.id(), PRINCIPAL, JARS_COUNT)
        .await?;

    jar::create_jar(&context.ft, &context.jar, alice, &product_10_min.id(), PRINCIPAL + 3)
        .await?
        .into_result()?;
    let jar_10_min = context.last_jar_for(alice).await?;
    assert_eq!(jar_10_min.principal.0, PRINCIPAL + 3);

    context.fast_forward_minutes(6).await?;

    // 2 calls to claim all 212 jars
    jar::claim_total(&context.jar, alice, None).await?;
    jar::claim_total(&context.jar, alice, None).await?;

    let alice_balance = ft::ft_balance_of(&context.ft, alice.id()).await?;
    let jar_balance = ft::ft_balance_of(&context.ft, context.jar.id()).await?;

    let withdrawn = jar::withdraw_all(&context.jar, alice).await?;
    assert_eq!(withdrawn.jars.len(), 200);

    let withdrawn_2 = jar::withdraw_all(&context.jar, alice).await?;
    assert_eq!(withdrawn_2.jars.len(), 12);

    let alice_balance_after = ft::ft_balance_of(&context.ft, alice.id()).await?;
    let jar_balance_after = ft::ft_balance_of(&context.ft, context.jar.id()).await?;

    assert_eq!(alice_balance_after - alice_balance, BULK_PRINCIPAL + 2_000_003);
    assert_eq!(jar_balance - jar_balance_after, BULK_PRINCIPAL + 2_000_003);

    assert_eq!(withdrawn.total_amount.0, 200_000_003);
    assert_eq!(withdrawn_2.total_amount.0, PRINCIPAL * 12);

    assert_eq!(
        withdrawn.jars.iter().map(|j| j.withdrawn_amount).collect::<Vec<_>>()[..2],
        vec![jar_5_min_1.principal, jar_5_min_2.principal]
    );

    let jars = jar::get_jars_for_account(&context.jar, alice.id()).await?;
    assert_eq!(jars.len(), 1);

    let jar_view = jars.into_iter().next().unwrap();
    assert_eq!(jar_view.id, jar_10_min.id);
    assert_eq!(jar_view.principal, jar_10_min.principal);

    Ok(())
}
