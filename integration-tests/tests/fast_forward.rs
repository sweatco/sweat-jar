use tracing::info;

mod common;
use common::{jar, prepare::prepare_contract};

#[tokio::test]
#[tracing::instrument]
async fn fast_forward() -> anyhow::Result<()> {
    common::prepare::init_tracing();
    info!("fast forward test");

    let context = prepare_contract([]).await?;

    let mut passed = vec![];

    for _ in 0..10 {
        let start_timestamp = jar::block_timestamp_ms(&context.jar).await?;
        context.fast_forward_minutes(1).await?;
        passed.push(jar::block_timestamp_ms(&context.jar).await? - start_timestamp);
    }

    let avg = passed.iter().sum::<u64>() / passed.len() as u64;
    info!("average ms advanced per fast_forward_minutes(1) call: {avg}");

    // Yeah this looks weird but workspace block skipping is very volatile
    assert!(52_000 < avg && avg < 76_000);

    Ok(())
}
