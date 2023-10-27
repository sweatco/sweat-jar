use near_sdk::Timestamp;

use crate::common::{prepare_contract, Prepared};

#[tokio::test]
#[mutants::skip]
async fn fast_forward() -> anyhow::Result<()> {
    println!("👷🏽 Run fast forward test");

    let Prepared {
        context,
        manager: _,
        alice: _,
        fee_account: _,
    } = prepare_contract([]).await?;

    let mut passed = vec![];

    for _ in 0..10 {
        let start_timestamp = context.jar_contract.block_timestamp_ms().await?;
        context.fast_forward_minutes(1).await?;
        passed.push(context.jar_contract.block_timestamp_ms().await? - start_timestamp)
    }

    let avg = passed.iter().sum::<Timestamp>() / passed.len() as Timestamp;

    // Yeah this looks weird but workspace block skipping is very volatile
    assert!(52_000 < avg && avg < 72_000);

    Ok(())
}
