//! Single-user reconciliation: replay a user's timeline and compare the
//! calculated total claim against the on-chain total.

use anyhow::{Context, Result};
use sweat_jar::replay::engine::{self, ReplayStatus};
use sweat_jar_model::data::product::Product;

use crate::parse;
use crate::snapshot::SnapshotSource;
use crate::timeline::{self, UserSlice};

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
fn zero_calc_row(slice: &UserSlice, status: impl Into<String>) -> ReconRow {
    let actual = slice.onchain_claimed;
    let delta: i128 = -i128::try_from(actual).unwrap_or(i128::MAX);
    ReconRow {
        account_id: slice.backend_account_id,
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
    conn: &duckdb::Connection,
    backend_account_id: i64,
    products: &[Product],
    snapshot: &dyn SnapshotSource,
) -> Result<ReconRow> {
    let (slice, timeline) = timeline::load_user(conn, backend_account_id)?;
    let actual = slice.onchain_claimed;

    let baseline_raw: std::thread::Result<Result<Option<Vec<u8>>>> =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            snapshot.raw_account(backend_account_id, &slice.near_account_id)
        }));

    let raw_account = match baseline_raw {
        Err(_) => return Ok(zero_calc_row(&slice, "error:snapshot panic")),
        Ok(Err(e)) => {
            return Ok(zero_calc_row(&slice, format!("error:{}", truncate(&e.to_string()))));
        }
        Ok(Ok(v)) => v,
    };

    let no_baseline = slice.existed_at_start && raw_account.is_none();

    let account_id: near_sdk::AccountId = match slice.near_account_id.parse() {
        Ok(a) => a,
        Err(_) => return Ok(zero_calc_row(&slice, "error:invalid near_account_id")),
    };

    // `run_timeline` catches contract panics itself, but a second caught panic on
    // one worker thread has been observed to escape `run_timeline`'s internal
    // `catch_unwind` (near-sdk mock harness state after the upfront
    // `set_timezone`); this guard keeps one bad account from killing the worker —
    // it becomes an `error:` row like any other.
    let outcome = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine::run_timeline(
            engine::Baseline { account_id, raw_account, timezone_ms: slice.timezone_ms },
            products,
            parse::H_MS,
            timeline,
        )
    })) {
        Ok(o) => o,
        Err(e) => {
            let msg = e
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| e.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "engine panic".to_string());
            return Ok(zero_calc_row(&slice, format!("error:{}", truncate(&msg))));
        }
    };

    let calculated = outcome.total_claimed;
    let delta: i128 = i128::try_from(calculated).context("calculated_total_claim exceeds i128")?
        - i128::try_from(actual).context("actual_total_claim exceeds i128")?;
    let rel_delta = if actual == 0 { 0.0 } else { delta as f64 / actual as f64 };

    let status = match &outcome.status {
        ReplayStatus::Error(msg) if no_baseline && msg.contains("is not found") => {
            "no_baseline".to_string()
        }
        ReplayStatus::Error(msg) => format!("error:{}", truncate(msg)),
        ReplayStatus::Ok if no_baseline => "no_baseline".to_string(),
        ReplayStatus::Ok => "ok".to_string(),
    };

    Ok(ReconRow {
        account_id: slice.backend_account_id,
        near_account_id: slice.near_account_id,
        calculated_total_claim: calculated.to_string(),
        actual_total_claim: actual.to_string(),
        delta: delta.to_string(),
        rel_delta,
        n_claims: outcome.per_claim.len(),
        status,
    })
}
