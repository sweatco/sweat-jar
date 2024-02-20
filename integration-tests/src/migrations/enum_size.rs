use sweat_jar_model::api::TestIncreasedEnumSizeIntegration;

use crate::context::{prepare_contract, IntegrationContext};

#[tokio::test]
#[mutants::skip]
async fn enum_size() -> anyhow::Result<()> {
    println!("👷🏽 Run enum size test");

    let context = prepare_contract(None, []).await?;

    context.sweat_jar().store_small_enum().await?;
    context.sweat_jar().migrate_to_big_enum().await?;
    context.sweat_jar().check_big_enum().await?;

    Ok(())
}
