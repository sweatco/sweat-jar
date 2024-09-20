use anyhow::Result;
use nitka::{misc::ToNear, set_integration_logs_enabled};
use sweat_jar_model::api::{ClaimApiIntegration, IntegrationTestMethodsIntegration, JarApiIntegration};

use crate::{
    context::{prepare_contract, IntegrationContext},
    product::RegisterProductCommand::Locked5Minutes60000Percents,
};

#[tokio::test]
#[mutants::skip]
async fn claim_many_jars() -> Result<()> {
    println!("👷🏽 Claim many jars test");

    set_integration_logs_enabled(false);

    let mut context = prepare_contract(None, [Locked5Minutes60000Percents]).await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;

    context
        .sweat_jar()
        .bulk_create_jars(alice.to_near(), Locked5Minutes60000Percents.id(), 1000, 4000)
        .with_user(&manager)
        .await?;

    dbg!(context.sweat_jar().get_jars_for_account(alice.to_near()).await?.len());

    context.fast_forward_minutes(5).await?;

    let claimed = context.sweat_jar().claim_total(true.into()).with_user(&alice).await?;

    let batch_claim_summ = claimed.get_total().0;

    dbg!(&batch_claim_summ);

    assert_eq!(
        batch_claim_summ * 39,
        context
            .sweat_jar()
            .get_total_interest(alice.to_near())
            .await?
            .amount
            .total
            .0
    );

    for i in 1..40 {
        let claimed = context.sweat_jar().claim_total(true.into()).with_user(&alice).await?;
        assert_eq!(claimed.get_total().0, batch_claim_summ);

        assert_eq!(
            batch_claim_summ * (39 - i),
            context
                .sweat_jar()
                .get_total_interest(alice.to_near())
                .await?
                .amount
                .total
                .0
        );
    }

    assert_eq!(
        context
            .sweat_jar()
            .get_total_interest(alice.to_near())
            .await?
            .amount
            .total
            .0,
        0
    );

    Ok(())
}
