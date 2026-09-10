//! Event-export loaders for the replay DuckDB database.
//!
//! The real `build_db` is implemented in Task 3 of
//! `docs/superpowers/plans/2026-09-10-event-sourced-replay.md`.

use std::path::Path;

use anyhow::Context;

/// Options controlling which accounts are ingested.
pub struct BuildOpts<'a> {
    pub source_dir: &'a std::path::Path,
    pub accounts: Option<&'a std::collections::HashSet<i64>>,
    pub sample: Option<usize>,
}

/// Ingest the on-chain event export into `conn` and return `(table, rows)` in
/// load order.
pub fn build_db(
    _c: &mut duckdb::Connection,
    _o: &BuildOpts,
) -> anyhow::Result<Vec<(String, i64)>> {
    anyhow::bail!("build_db: implemented in Task 3")
}

/// Parse an account-id list file: one `account_id` per line, blank lines and
/// `#` comments ignored. Shared by `build-db --accounts` and `run --accounts`.
pub fn read_accounts(path: &Path) -> anyhow::Result<Vec<i64>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read accounts file {}", path.display()))?;
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.parse::<i64>().with_context(|| format!("parse account_id {l:?}")))
        .collect()
}
