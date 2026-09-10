//! Single-user reconciliation: replay a user's timeline and compare the
//! calculated total claim against the on-chain total.
//!
//! Rewritten in Task 6 of
//! `docs/superpowers/plans/2026-09-10-event-sourced-replay.md`.

use anyhow::{bail, Result};
use sweat_jar_model::data::product::Product;

use crate::snapshot::SnapshotSource;

/// One reconciliation result row.
#[derive(Debug, serde::Serialize)]
pub struct ReconRow {
    pub account_id: i64,
    pub near_account_id: String,
    pub calculated_total_claim: String,
    pub actual_total_claim: String,
    pub delta: String,
    pub rel_delta: f64,
    pub n_claims: usize,
    pub status: String,
}

/// Reconcile one user. Never panics — a panic in snapshot parsing or the engine
/// becomes `status = "error:<msg>"`. Returns `Err` only for a DB/IO failure that
/// isn't user-specific.
pub fn reconcile_user(
    _conn: &duckdb::Connection,
    _account_id: i64,
    _products: &[Product],
    _snapshot: &dyn SnapshotSource,
) -> Result<ReconRow> {
    bail!("reconcile_user: rewritten in Task 6 of docs/superpowers/plans/2026-09-10-event-sourced-replay.md")
}
