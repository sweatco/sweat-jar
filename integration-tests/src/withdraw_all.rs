use nitka::{misc::ToNear, set_integration_logs_enabled};
use sweat_jar_model::api::{ClaimApiIntegration, JarApiIntegration, WithdrawApiIntegration};

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
    product::RegisterProductCommand,
};

#[tokio::test]
#[mutants::skip]
async fn withdraw_all() -> anyhow::Result<()> {
    const PRINCIPAL: u128 = 1_000_000;

    println!("👷🏽 Run test for withdraw all");

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

    let claimed = context.sweat_jar().claim_total(None).await?;
    assert_eq!(claimed.get_total().0, 0);

    context.fast_forward_minutes(6).await?;

    context.sweat_jar().claim_total(None).with_user(&alice).await?;

    let withdrawn = context.sweat_jar().withdraw_all().with_user(&alice).await?;

    assert_eq!(withdrawn.total_amount.0, 2000003);

    assert_eq!(
        withdrawn.jars.iter().map(|j| j.withdrawn_amount).collect::<Vec<_>>(),
        vec![jar_5_min_1.principal, jar_5_min_2.principal]
    );

    let jars = context.sweat_jar().get_jars_for_account(alice.to_near()).await?;

    assert_eq!(jars.len(), 1);

    let jar = jars.into_iter().next().unwrap();

    assert_eq!(jar.id, jar_10_min.id);
    assert_eq!(jar.principal, jar_10_min.principal);

    Ok(())
}
