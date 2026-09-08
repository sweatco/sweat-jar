//! CSV -> SQLite loaders for the replay database.

use std::{collections::HashSet, path::Path};

use anyhow::{bail, Context};

use crate::{db, parse};

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

/// Ingest the requested tables. `users` is loaded first when in scope; the other
/// tables filter their rows against the `keep` account set.
pub fn build_db(conn: &mut rusqlite::Connection, opts: &BuildOpts) -> anyhow::Result<()> {
    let dir = opts.test_data_dir;

    // Compute the keep set once.
    let owned_keep: Option<HashSet<i64>> = if opts.accounts.is_some() {
        None
    } else if let Some(n) = opts.sample {
        Some(first_n_account_ids(dir, n)?)
    } else {
        None
    };
    let keep: Option<&HashSet<i64>> = opts.accounts.or(owned_keep.as_ref());

    if opts.wants("users") {
        ingest_users(conn, opts)?;
    }
    if opts.wants("subscriptions") {
        ingest_subscriptions(conn, keep, dir)?;
    }
    if opts.wants("jar_events") {
        println!("jar_events ingest: not yet implemented (task 8)");
    }
    if opts.wants("step_packages") {
        println!("step_packages ingest: not yet implemented (task 8)");
    }
    if opts.wants("snapshots") {
        println!("snapshots ingest: not yet implemented (task 9)");
    }

    if opts.wants("jar_events") || opts.wants("step_packages") || opts.wants("subscriptions") {
        db::schema::create_indexes(conn)?;
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
/// is dropped. Returns the effective keep set (`None` when no filter applies).
fn ingest_users(
    conn: &mut rusqlite::Connection,
    opts: &BuildOpts,
) -> anyhow::Result<Option<HashSet<i64>>> {
    let keep: Option<HashSet<i64>> = if let Some(accounts) = opts.accounts {
        Some(accounts.clone())
    } else {
        opts.sample
            .map(|n| first_n_account_ids(opts.test_data_dir, n))
            .transpose()?
    };

    let path = opts.test_data_dir.join("users.csv");
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&path)
        .with_context(|| format!("open {}", path.display()))?;

    let tx = conn.transaction()?;
    {
        let mut stmt =
            tx.prepare("INSERT OR REPLACE INTO users (account_id, near_account_id) VALUES (?1, ?2)")?;
        for rec in rdr.records() {
            let rec = rec?;
            let account_id: i64 = rec[0].trim().parse().context("parse account_id")?;
            if let Some(k) = &keep {
                if !k.contains(&account_id) {
                    continue;
                }
            }
            let near = rec[1].trim();
            stmt.execute(rusqlite::params![account_id, near])?;
        }
    }
    tx.commit()?;
    Ok(keep)
}

/// Load `max_subscriptions.csv` -> `subscriptions(account_id, ts_ms, active)`.
fn ingest_subscriptions(
    conn: &mut rusqlite::Connection,
    keep: Option<&HashSet<i64>>,
    dir: &Path,
) -> anyhow::Result<()> {
    let path = dir.join("max_subscriptions.csv");
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&path)
        .with_context(|| format!("open {}", path.display()))?;

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
                .with_context(|| format!("subscription datetime for account {account_id}"))?
                as i64;
            let active: i64 = match rec[2].trim() {
                "subscribed" => 1,
                "expired" => 0,
                other => bail!("unknown subscription action_type: {other:?}"),
            };
            stmt.execute(rusqlite::params![account_id, ts_ms, active])?;
        }
    }
    tx.commit()?;
    Ok(())
}
