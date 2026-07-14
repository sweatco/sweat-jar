mod common;

use anyhow::Result;
use common::{jar, prepare::prepare_contract};

#[tokio::test]
async fn fast_forward() -> Result<()> {
    let context = prepare_contract([]).await?;

    let mut passed = vec![];

    for _ in 0..10 {
        let start_timestamp = jar::block_timestamp_ms(&context.jar).await?;
        context.fast_forward_minutes(1).await?;
        passed.push(jar::block_timestamp_ms(&context.jar).await? - start_timestamp);
    }

    let avg = passed.iter().sum::<u64>() / passed.len() as u64;

    // Sandbox block skipping is very volatile, hence the wide bounds.
    assert!(52_000 < avg && avg < 76_000, "avg minute skip was {avg} ms");

    Ok(())
}
