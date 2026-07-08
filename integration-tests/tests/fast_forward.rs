use anyhow::Result;
use tracing::info;

mod common;
use common::{jar, prepare::prepare_contract};

const TOLERANCE_MS: std::ops::Range<u64> = 52_000..76_000;
const MAX_ATTEMPTS: u32 = 3;

/// Sandbox block-skip timing is volatile enough that a single 10-sample
/// average can land outside `TOLERANCE_MS` by chance alone (see the
/// `BLOCKS_PER_MINUTE` calibration comment in `common/prepare.rs`). Retry a
/// few times before failing so ordinary sandbox variance doesn't produce a
/// spurious CI failure — a real miscalibration will still fail consistently.
#[tokio::test]
#[tracing::instrument]
async fn fast_forward() -> Result<()> {
    common::prepare::init_tracing();
    info!("fast forward test");

    let context = prepare_contract([]).await?;

    let mut avg = 0;

    for attempt in 1..=MAX_ATTEMPTS {
        let mut passed = vec![];

        for _ in 0..10 {
            let start_timestamp = jar::block_timestamp_ms(&context.jar).await?;
            context.fast_forward_minutes(1).await?;
            passed.push(jar::block_timestamp_ms(&context.jar).await? - start_timestamp);
        }

        avg = passed.iter().sum::<u64>() / passed.len() as u64;
        info!("attempt {attempt}: average ms advanced per fast_forward_minutes(1) call: {avg}");

        if TOLERANCE_MS.contains(&avg) {
            return Ok(());
        }
    }

    panic!(
        "average ms advanced per fast_forward_minutes(1) call ({avg}) fell outside \
         {TOLERANCE_MS:?} in every one of {MAX_ATTEMPTS} attempts"
    );
}
