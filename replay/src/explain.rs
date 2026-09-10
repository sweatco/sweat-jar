//! `explain` — replay one account and print its per-claim breakdown next to the
//! on-chain claim amounts, so a non-zero `delta` can be traced to the claim that
//! diverged.
//!
//! Rewritten in Task 7 of
//! `docs/superpowers/plans/2026-09-10-event-sourced-replay.md`.

use std::path::PathBuf;

use anyhow::Result;

pub struct ExplainOpts {
    pub db: PathBuf,
    pub account: i64,
    pub products: PathBuf,
    pub archival_rpc_url: Option<String>,
}

pub fn explain(_opts: &ExplainOpts) -> Result<()> {
    anyhow::bail!("explain: rewritten in Task 7 of docs/superpowers/plans/2026-09-10-event-sourced-replay.md")
}
