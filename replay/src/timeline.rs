//! Per-user DB slice -> engine [`Timeline`] synthesis.
//!
//! Rewritten in Task 5 of
//! `docs/superpowers/plans/2026-09-10-event-sourced-replay.md`.

use anyhow::{bail, Result};
use duckdb::Connection;
use sweat_jar::replay::engine::Timeline;

/// One account's on-chain summary alongside its synthesized timeline.
pub struct UserSlice {
    pub account_id: i64,
    pub near_account_id: String,
    /// Sum of every claim the account made on-chain in the replay window.
    pub onchain_claimed: u128,
}

/// Reads every event row for `account_id` and builds a sorted engine
/// [`Timeline`].
pub fn load_user(_conn: &Connection, _account_id: i64) -> Result<(UserSlice, Timeline)> {
    bail!("timeline::load_user: rewritten in Task 5 of docs/superpowers/plans/2026-09-10-event-sourced-replay.md")
}
