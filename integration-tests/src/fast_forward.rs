use nitka::near_sdk::Timestamp;

use crate::{
    context::{prepare_contract, IntegrationContext},
    jar_contract_extensions::JarContractExtensions,
};

#[tokio::test]
#[mutants::skip]
async fn fast_forward() -> anyhow::Result<()> {
    println!("👷🏽 Run fast forward test");

    let context = prepare_contract(None, []).await?;

    let mut passed = vec![];

    for _ in 0..10 {
        let start_timestamp = context.sweat_jar().block_timestamp_ms().await?;
        context.fast_forward_minutes(1).await?;
        passed.push(context.sweat_jar().block_timestamp_ms().await? - start_timestamp)
    }

    let avg = passed.iter().sum::<Timestamp>() / passed.len() as Timestamp;

    // Yeah this looks weird but workspace block skipping is very volatile
    assert!(52_000 < avg && avg < 76_000);

    Ok(())
}
