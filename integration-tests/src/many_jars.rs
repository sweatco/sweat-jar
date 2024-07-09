use anyhow::Result;
use nitka::misc::ToNear;
use sweat_jar_model::api::{ClaimApiIntegration, IntegrationTestMethodsIntegration, JarApiIntegration};

use crate::{
    context::{prepare_contract, IntegrationContext},
    product::RegisterProductCommand::Locked5Minutes60000Percents,
};

#[tokio::test]
#[mutants::skip]
async fn claim_many_jars() -> Result<()> {
    println!("👷🏽 Claim many jars test");

    let mut context = prepare_contract(None, [Locked5Minutes60000Percents]).await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;

    context
        .sweat_jar()
        .bulk_create_jars(alice.to_near(), Locked5Minutes60000Percents.id(), 10000, 450)
        .with_user(&manager)
        .await?;

    context.fast_forward_minutes(5).await?;

    context
        .sweat_jar()
        .claim_total(true.into())
        .with_user(&alice)
        .result()
        .await?;

    assert!(context
        .sweat_jar()
        .get_jars_for_account(alice.to_near())
        .await?
        .iter()
        .all(|j| j.is_pending_withdraw == false));

    Ok(())
}
