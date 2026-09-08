//! Single-user reconciliation: replay a user's timeline and compare the
//! calculated total claim against the on-chain total.

use anyhow::{Context, Result};
use sweat_jar::replay::engine::{self, ReplayStatus};
use sweat_jar_model::data::product::Product;

use crate::snapshot::SnapshotSource;
use crate::timeline::{load_user, UserSlice};
use crate::parse;

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

fn truncate(s: &str) -> String {
    if s.chars().count() <= 120 {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(120).collect::<String>())
    }
}

/// A row with `calculated = 0` for the error / no-snapshot-yet paths.
/// Per-user yocto totals stay far under `i128::MAX`; the saturating cast is a
/// last-resort guard, not an expected path.
fn zero_calc_row(slice: &UserSlice, status: impl Into<String>) -> ReconRow {
    let actual = slice.onchain_claimed;
    let delta: i128 = -i128::try_from(actual).unwrap_or(i128::MAX);
    ReconRow {
        account_id: slice.account_id,
        near_account_id: slice.near_account_id.clone(),
        calculated_total_claim: "0".to_string(),
        actual_total_claim: actual.to_string(),
        delta: delta.to_string(),
        rel_delta: if actual == 0 { 0.0 } else { delta as f64 / actual as f64 },
        n_claims: 0,
        status: status.into(),
    }
}

/// Reconcile one user. Never panics — a panic in snapshot parsing or the engine
/// becomes `status = "error:<msg>"`. Returns `Err` only for a DB/IO failure that
/// isn't user-specific.
pub fn reconcile_user(
    conn: &rusqlite::Connection,
    account_id: i64,
    products: &[Product],
    snapshot: &dyn SnapshotSource,
) -> Result<ReconRow> {
    let (slice, timeline) = load_user(conn, account_id)?;
    let actual = slice.onchain_claimed;

    let baseline_raw: std::thread::Result<Result<Option<Vec<u8>>>> =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| snapshot.raw_account(account_id)));

    let (raw_account, no_baseline) = match baseline_raw {
        Err(_) => return Ok(zero_calc_row(&slice, "error:snapshot panic")),
        Ok(Err(e)) => {
            return Ok(zero_calc_row(
                &slice,
                format!("error:{}", truncate(&e.to_string())),
            ));
        }
        Ok(Ok(None)) => (None, true),
        Ok(Ok(Some(bytes))) => (Some(bytes), false),
    };

    let account_id_str: near_sdk::AccountId = match slice.near_account_id.parse() {
        Ok(a) => a,
        Err(_) => return Ok(zero_calc_row(&slice, "error:invalid near_account_id")),
    };

    let baseline = engine::Baseline {
        account_id: account_id_str,
        raw_account,
    };

    let outcome = engine::run_timeline(baseline, products, parse::H_MS, timeline);

    let calculated = outcome.total_claimed;
    let delta: i128 = i128::try_from(calculated).context("calculated_total_claim exceeds i128")?
        - i128::try_from(actual).context("actual_total_claim exceeds i128")?;
    let rel_delta = if actual == 0 { 0.0 } else { delta as f64 / actual as f64 };

    let status = match &outcome.status {
        ReplayStatus::Error(msg) => format!("error:{}", truncate(msg)),
        ReplayStatus::Ok if no_baseline => "no_baseline".to_string(),
        ReplayStatus::Ok => "ok".to_string(),
    };

    Ok(ReconRow {
        account_id,
        near_account_id: slice.near_account_id,
        calculated_total_claim: calculated.to_string(),
        actual_total_claim: actual.to_string(),
        delta: delta.to_string(),
        rel_delta,
        n_claims: outcome.per_claim.len(),
        status,
    })
}
