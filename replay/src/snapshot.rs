//! Baseline snapshot source: a user's account state at block H as borsh bytes
//! of an `AccountVersioned`, for the engine's `Baseline.raw_account`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use sweat_jar_model::data::account::{versioned::AccountVersioned, Account};

/// Source of a user's account state at block H (borsh bytes of an `AccountVersioned`), or `None`.
pub trait SnapshotSource: Send + Sync {
    /// `account_id` is the integer replay id. Returns the raw borsh account bytes for the
    /// engine's `Baseline.raw_account`, or `Ok(None)` if no snapshot exists for this account.
    fn raw_account(&self, account_id: i64) -> Result<Option<Vec<u8>>>;
}

/// Reads the `snapshots` table of a replay DB.
pub struct DbSnapshotSource {
    db_path: PathBuf,
}

impl DbSnapshotSource {
    pub fn new(db_path: impl Into<PathBuf>) -> Self {
        Self { db_path: db_path.into() }
    }
}

thread_local! {
    /// One read-only SQLite connection per worker thread, reused across the
    /// ~1.2M `raw_account` calls a run makes. Keyed by db path so a thread that
    /// somehow sees two different paths reopens rather than querying the wrong DB.
    static CONN: RefCell<Option<(PathBuf, rusqlite::Connection)>> = const { RefCell::new(None) };
}

impl DbSnapshotSource {
    /// Run `f` with this thread's cached connection for `db_path`, opening one on
    /// first use (or when the path changed).
    fn with_conn<T>(
        db_path: &Path,
        f: impl FnOnce(&rusqlite::Connection) -> Result<T>,
    ) -> Result<T> {
        CONN.with(|cell| {
            let mut slot = cell.borrow_mut();
            let needs_open = !matches!(&*slot, Some((p, _)) if p == db_path);
            if needs_open {
                let conn = crate::db::open_read(db_path)?;
                *slot = Some((db_path.to_path_buf(), conn));
            }
            let (_, conn) = slot.as_ref().expect("connection just set");
            f(conn)
        })
    }
}

impl SnapshotSource for DbSnapshotSource {
    fn raw_account(&self, account_id: i64) -> Result<Option<Vec<u8>>> {
        let line: Option<String> = Self::with_conn(&self.db_path, |conn| {
            conn.query_row(
                "SELECT state_json FROM snapshots WHERE account_id = ?1",
                [account_id],
                |r| r.get(0),
            )
            .optional()
            .context("querying snapshots table")
        })?;

        let Some(line) = line else { return Ok(None) };

        let v: near_sdk::serde_json::Value =
            near_sdk::serde_json::from_str(&line).context("snapshot row is not valid JSON")?;
        let account_state = v
            .get("account_state")
            .context("snapshot row has no `account_state` field")?;
        Ok(Some(account_state_json_to_raw(&account_state.to_string())?))
    }
}

/// Unimplemented stub for the future archival-RPC path.
pub struct ArchivalRpcSnapshotSource {
    pub rpc_url: String,
    pub block_height: u64,
}

impl SnapshotSource for ArchivalRpcSnapshotSource {
    fn raw_account(&self, _account_id: i64) -> Result<Option<Vec<u8>>> {
        anyhow::bail!(
            "ArchivalRpcSnapshotSource is not implemented yet (block_height={}, rpc={})",
            self.block_height,
            self.rpc_url
        )
    }
}

/// `account_state` JSON (as a string) -> borsh(`AccountVersioned`) bytes.
/// `crate::parse::H_MS` is the `score.updated_at` fallback (see `engine::parse_account_state`).
pub fn account_state_json_to_raw(state_json: &str) -> Result<Vec<u8>> {
    let v: near_sdk::serde_json::Value =
        near_sdk::serde_json::from_str(state_json).context("snapshot account_state is not valid JSON")?;
    let account: Account = sweat_jar::replay::engine::parse_account_state(&v, crate::parse::H_MS);
    near_sdk::borsh::to_vec(&AccountVersioned::new(account)).context("borsh-encode AccountVersioned")
}

#[cfg(test)]
mod tests {
    use near_sdk::borsh::BorshDeserialize;
    use sweat_jar_model::data::account::versioned::AccountVersioned;

    use super::*;
    use crate::db::{
        self,
        ingest::{build_db, BuildOpts},
    };

    #[test]
    fn json_snapshot_round_trips_to_borsh() {
        let json = std::fs::read_to_string("tests/fixtures/snapshots.ndjson").unwrap();
        let line = json.lines().next().unwrap();
        let v: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(line).unwrap();
        let raw = account_state_json_to_raw(&v["account_state"].to_string()).unwrap();
        assert!(!raw.is_empty());
        AccountVersioned::try_from_slice(&raw).unwrap();
    }

    #[test]
    fn archival_rpc_source_is_unimplemented() {
        let err = ArchivalRpcSnapshotSource {
            rpc_url: "x".into(),
            block_height: 1,
        }
        .raw_account(1)
        .unwrap_err();
        assert!(err.to_string().contains("not implemented"));
    }

    #[test]
    fn parse_account_state_stamps_zero_updated_at_to_window_start() {
        let v: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(
            r#"{"jars":{},"score":{"updated_at":0,"history":[{"value":10,"booster":0}]},"features":{}}"#,
        )
        .unwrap();
        let account = sweat_jar::replay::engine::parse_account_state(&v, crate::parse::H_MS);
        assert_eq!(account.score.updated_at(), crate::parse::H_MS);
    }

    #[test]
    fn parse_account_state_stamps_missing_updated_at_to_window_start() {
        let v: near_sdk::serde_json::Value =
            near_sdk::serde_json::from_str(r#"{"jars":{},"score":{"history":[]},"features":{}}"#).unwrap();
        let account = sweat_jar::replay::engine::parse_account_state(&v, crate::parse::H_MS);
        assert_eq!(account.score.updated_at(), crate::parse::H_MS);
    }

    #[test]
    fn db_snapshot_source_reads_table() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let mut conn = db::open_write(&path).unwrap();
        db::schema::init_schema(&conn).unwrap();
        build_db(
            &mut conn,
            &BuildOpts {
                test_data_dir: std::path::Path::new("tests/fixtures"),
                only: &["users".into(), "snapshots".into()],
                accounts: None,
                sample: None,
            },
        )
        .unwrap();
        drop(conn);

        let src = DbSnapshotSource::new(&path);
        let bytes = src.raw_account(36988193).unwrap().expect("snapshot present");
        AccountVersioned::try_from_slice(&bytes).unwrap();
        assert!(src.raw_account(999999).unwrap().is_none());
    }
}
