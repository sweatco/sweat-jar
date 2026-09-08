//! CSV -> SQLite loaders for the replay database.

use std::{collections::HashSet, path::Path};

use anyhow::{bail, Context};
use rusqlite::OptionalExtension;

use crate::{db, parse};

/// Tables that `build_db` knows how to ingest. An `only` value outside this set
/// is a typo and is rejected.
const TABLES: [&str; 5] = ["users", "jar_events", "step_packages", "subscriptions", "snapshots"];

/// Commit and reopen the transaction every this many inserted `step_packages`
/// rows to bound journal/statement-cache growth on the ~285M-row table.
const BATCH: usize = 1_000_000;

/// Options controlling which tables are ingested and which accounts are kept.
pub struct BuildOpts<'a> {
    pub test_data_dir: &'a Path,
    /// Table names to ingest; empty means all.
    pub only: &'a [String],
    /// Explicit account allow-list. Takes precedence over `sample`.
    pub accounts: Option<&'a HashSet<i64>>,
    /// Keep only the first `n` accounts from `users.csv` (file order).
    pub sample: Option<usize>,
}

impl BuildOpts<'_> {
    fn wants(&self, table: &str) -> bool {
        self.only.is_empty() || self.only.iter().any(|t| t == table)
    }
}

/// Ingest the requested tables and return `(table, rows_inserted)` in load order.
///
/// `users` is always loaded before `subscriptions`/`jar_events`/`step_packages`/
/// `snapshots` when it is in scope — those tables need the keep set, and
/// `snapshots` joins `near_account_id` against the `users` table. When `only`
/// selects `snapshots` without `users`, `snapshots` resolves against whatever
/// `users` rows already exist in the DB (no rows are force-added).
pub fn build_db(
    conn: &mut rusqlite::Connection,
    opts: &BuildOpts,
) -> anyhow::Result<Vec<(&'static str, usize)>> {
    let dir = opts.test_data_dir;

    for t in opts.only {
        if !TABLES.contains(&t.as_str()) {
            bail!("unknown --only table: {t:?} (expected one of {TABLES:?})");
        }
    }

    // Compute the keep set once — final, single-sourced. Threaded into every
    // per-account loader.
    let owned_keep: Option<HashSet<i64>> = if opts.accounts.is_some() {
        None
    } else if let Some(n) = opts.sample {
        Some(first_n_account_ids(dir, n)?)
    } else {
        None
    };
    let keep: Option<&HashSet<i64>> = opts.accounts.or(owned_keep.as_ref());

    let mut counts: Vec<(&'static str, usize)> = Vec::new();

    if opts.wants("users") {
        counts.push(("users", ingest_users(conn, keep, dir)?));
    }
    if opts.wants("subscriptions") {
        counts.push(("subscriptions", ingest_subscriptions(conn, keep, dir)?));
    }
    if opts.wants("jar_events") {
        counts.push(("jar_events", ingest_jar_events(conn, keep, dir)?));
    }
    if opts.wants("step_packages") {
        counts.push(("step_packages", ingest_step_packages(conn, keep, dir)?));
    }
    if opts.wants("snapshots") {
        counts.push(("snapshots", ingest_snapshots(conn, keep, dir)?));
    }

    db::schema::create_indexes(conn)?;

    write_meta(conn)?;

    Ok(counts)
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

/// Record the replay window bounds and build time into `meta`.
fn write_meta(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    let built_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "0".into());
    let rows = [
        ("window_h_ms", parse::H_MS.to_string()),
        ("window_t_end_ms", parse::T_END_MS.to_string()),
        ("built_at", built_at),
    ];
    for (k, v) in rows {
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            rusqlite::params![k, v],
        )?;
    }
    Ok(())
}

/// First `n` `account_id`s from `users.csv`, in file order.
fn first_n_account_ids(dir: &Path, n: usize) -> anyhow::Result<HashSet<i64>> {
    let path = dir.join("users.csv");
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&path)
        .with_context(|| format!("open {}", path.display()))?;
    let mut out = HashSet::new();
    for rec in rdr.records().take(n) {
        let rec = rec?;
        out.insert(rec[0].trim().parse::<i64>().context("parse account_id")?);
    }
    Ok(out)
}

/// Load `users.csv` -> `users(account_id, near_account_id)`. `sweatcoin_user_id`
/// is dropped. Rows outside `keep` (when a filter is active) are skipped.
fn ingest_users(
    conn: &mut rusqlite::Connection,
    keep: Option<&HashSet<i64>>,
    dir: &Path,
) -> anyhow::Result<usize> {
    let path = dir.join("users.csv");
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&path)
        .with_context(|| format!("open {}", path.display()))?;

    let mut inserted = 0usize;
    let tx = conn.transaction()?;
    {
        let mut stmt =
            tx.prepare("INSERT OR REPLACE INTO users (account_id, near_account_id) VALUES (?1, ?2)")?;
        for rec in rdr.records() {
            let rec = rec?;
            let account_id: i64 = rec[0].trim().parse().context("parse account_id")?;
            if let Some(k) = keep {
                if !k.contains(&account_id) {
                    continue;
                }
            }
            let near = rec[1].trim();
            stmt.execute(rusqlite::params![account_id, near])?;
            inserted += 1;
        }
    }
    tx.commit()?;
    Ok(inserted)
}

/// Load `jar_events.csv` -> `jar_events(account_id, ts_ms, seq, event_type, product_id, amount)`.
///
/// Header: `account_id,jar_id,product_id,product_name,near_block_timestamp,event_type,amount,fee_amount,deposit_ids`.
/// `seq` is a 0-based counter over every data row in file order, assigned before
/// filtering so it is a stable global tie-breaker. Rows are kept only when the
/// event lands inside `(H_MS, T_END_MS]`, the `event_type` is one of
/// `{deposit, claim, withdraw, restake}` (`merge` is dropped), and — when `keep`
/// is set — the `account_id` is in it. `amount` is stored as the raw CSV string.
fn ingest_jar_events(
    conn: &mut rusqlite::Connection,
    keep: Option<&HashSet<i64>>,
    dir: &Path,
) -> anyhow::Result<usize> {
    let path = dir.join("jar_events.csv");
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&path)
        .with_context(|| format!("open {}", path.display()))?;

    let mut inserted = 0usize;
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO jar_events (account_id, ts_ms, seq, event_type, product_id, amount) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for (seq, rec) in rdr.records().enumerate() {
            let rec = rec?;
            let seq = seq as i64;

            let account_id: i64 = rec[0].trim().parse().context("parse account_id")?;

            let event_type = rec[5].trim();
            match event_type {
                "deposit" | "claim" | "withdraw" | "restake" => {}
                "merge" => continue,
                other => bail!("unknown jar_events event_type: {other:?}"),
            }

            let ts_ms = parse::iso8601_ms_to_epoch_ms(rec[4].trim())
                .with_context(|| format!("jar_events timestamp for account {account_id}"))?;
            if !(parse::H_MS < ts_ms && ts_ms <= parse::T_END_MS) {
                continue;
            }

            if let Some(k) = keep {
                if !k.contains(&account_id) {
                    continue;
                }
            }

            let product_id = rec[2].trim();
            let amount = rec[6].trim();
            stmt.execute(rusqlite::params![
                account_id,
                ts_ms as i64,
                seq,
                event_type,
                product_id,
                amount
            ])?;
            inserted += 1;
        }
    }
    tx.commit()?;
    Ok(inserted)
}

/// Load `max_subscriptions.csv` -> `subscriptions(account_id, ts_ms, active)`.
///
/// Rows are kept only when `ts_ms` lands inside `(H_MS, T_END_MS]`.
fn ingest_subscriptions(
    conn: &mut rusqlite::Connection,
    keep: Option<&HashSet<i64>>,
    dir: &Path,
) -> anyhow::Result<usize> {
    let path = dir.join("max_subscriptions.csv");
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&path)
        .with_context(|| format!("open {}", path.display()))?;

    let mut inserted = 0usize;
    let tx = conn.transaction()?;
    {
        let mut stmt = tx
            .prepare("INSERT INTO subscriptions (account_id, ts_ms, active) VALUES (?1, ?2, ?3)")?;
        for rec in rdr.records() {
            let rec = rec?;
            let account_id: i64 = rec[0].trim().parse().context("parse user_id")?;
            if let Some(k) = keep {
                if !k.contains(&account_id) {
                    continue;
                }
            }
            let ts_ms = parse::iso8601_ms_to_epoch_ms(rec[1].trim())
                .with_context(|| format!("subscription datetime for account {account_id}"))?;
            if !(parse::H_MS < ts_ms && ts_ms <= parse::T_END_MS) {
                continue;
            }
            let ts_ms = ts_ms as i64;
            let active: i64 = match rec[2].trim() {
                "subscribed" => 1,
                "expired" => 0,
                other => bail!("unknown subscription action_type: {other:?}"),
            };
            stmt.execute(rusqlite::params![account_id, ts_ms, active])?;
            inserted += 1;
        }
    }
    tx.commit()?;
    Ok(inserted)
}

/// Load `step_packages.csv` -> `step_packages(account_id, ts_ms, steps)`.
///
/// Header: `account_id,created_at,steps`. `created_at` is `"YYYY-MM-DD HH:MM:SS UTC"`.
/// Rows are kept only when `ts_ms` lands inside `(H_MS, T_END_MS]` and — when
/// `keep` is set — the `account_id` is in it. `steps` is parsed as `i64`
/// (negative is an error) then clamped to `u16::MAX` (65535).
///
/// This is the largest input (~285M rows), so the transaction is committed and
/// reopened every `BATCH` inserted rows to bound journal/statement-cache growth.
fn ingest_step_packages(
    conn: &mut rusqlite::Connection,
    keep: Option<&HashSet<i64>>,
    dir: &Path,
) -> anyhow::Result<usize> {
    // Tests override the batch size to exercise the commit-boundary seam.
    let batch = std::env::var("REPLAY_STEP_BATCH")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(BATCH);
    ingest_step_packages_with_batch(conn, keep, dir, batch)
}

fn ingest_step_packages_with_batch(
    conn: &mut rusqlite::Connection,
    keep: Option<&HashSet<i64>>,
    dir: &Path,
    batch: usize,
) -> anyhow::Result<usize> {
    let path = dir.join("step_packages.csv");
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&path)
        .with_context(|| format!("open {}", path.display()))?;

    let mut inserted = 0usize;
    let mut since_commit = 0usize;
    let mut tx = conn.transaction()?;
    for rec in rdr.records() {
        let rec = rec?;
        let account_id: i64 = rec[0].trim().parse().context("parse account_id")?;

        let ts_ms = parse::space_utc_to_epoch_ms(rec[1].trim())
            .with_context(|| format!("step_packages created_at for account {account_id}"))?;
        if !(parse::H_MS < ts_ms && ts_ms <= parse::T_END_MS) {
            continue;
        }

        if let Some(k) = keep {
            if !k.contains(&account_id) {
                continue;
            }
        }

        let steps_raw: i64 = rec[2].trim().parse().context("parse steps")?;
        if steps_raw < 0 {
            bail!("negative steps for account {account_id}: {steps_raw}");
        }
        let steps = steps_raw.min(65535);

        // prepare_cached keys off the Connection cache, which survives the
        // per-batch commit/reopen cycle — one reused compiled statement across
        // the whole ~285M-row load.
        tx.prepare_cached("INSERT INTO step_packages (account_id, ts_ms, steps) VALUES (?1, ?2, ?3)")?
            .execute(rusqlite::params![account_id, ts_ms as i64, steps])?;
        inserted += 1;
        since_commit += 1;

        if since_commit >= batch {
            tx.commit()?;
            tx = conn.transaction()?;
            since_commit = 0;
        }
    }
    tx.commit()?;
    Ok(inserted)
}

/// Load `snapshots.ndjson` -> `snapshots(account_id, state_json)`.
///
/// One JSON object per line, each with a top-level `"near_account_id"` string.
/// `account_id` is resolved by looking that up in the `users` table, so `users`
/// must be ingested first. A line whose `near_account_id` is unknown is skipped
/// (count logged to stderr). The stored `state_json` is the verbatim raw line.
///
/// If `snapshots.ndjson` does not exist this is a no-op returning `Ok(0)` — the
/// archival-RPC extractor that produces it is not built yet.
fn ingest_snapshots(
    conn: &mut rusqlite::Connection,
    keep: Option<&HashSet<i64>>,
    dir: &Path,
) -> anyhow::Result<usize> {
    let path = dir.join("snapshots.ndjson");
    if !path.exists() {
        return Ok(0);
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;

    let mut inserted = 0usize;
    let mut skipped = 0usize;
    let tx = conn.transaction()?;
    {
        let mut lookup =
            tx.prepare("SELECT account_id FROM users WHERE near_account_id = ?1")?;
        let mut ins =
            tx.prepare("INSERT OR REPLACE INTO snapshots (account_id, state_json) VALUES (?1, ?2)")?;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let v: serde_json::Value =
                serde_json::from_str(line).context("parse snapshots.ndjson line")?;
            let near = v
                .get("near_account_id")
                .and_then(|n| n.as_str())
                .context("snapshots line missing near_account_id string")?;

            let account_id: Option<i64> = lookup
                .query_row(rusqlite::params![near], |r| r.get(0))
                .optional()?;
            let Some(account_id) = account_id else {
                skipped += 1;
                continue;
            };

            if let Some(k) = keep {
                if !k.contains(&account_id) {
                    continue;
                }
            }

            ins.execute(rusqlite::params![account_id, line])?;
            inserted += 1;
        }
    }
    tx.commit()?;
    if skipped > 0 {
        eprintln!("snapshots: skipped {skipped} lines with unknown near_account_id");
    }
    Ok(inserted)
}
