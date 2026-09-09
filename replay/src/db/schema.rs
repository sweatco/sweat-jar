//! DDL for the replay database. Kept verbatim from the design spec.

use anyhow::Context;
use rusqlite::Connection;

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS users (
    account_id       INTEGER PRIMARY KEY,
    near_account_id  TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS jar_events (
    account_id  INTEGER NOT NULL,
    ts_ms       INTEGER NOT NULL,
    seq         INTEGER NOT NULL,
    event_type  TEXT NOT NULL,
    product_id  TEXT NOT NULL,
    amount      TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS step_packages (
    account_id       INTEGER NOT NULL,
    ts_ms            INTEGER NOT NULL,
    steps            INTEGER NOT NULL,
    yesterday_steps  INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS boosted_step_packages (
    account_id  INTEGER NOT NULL,
    ts_ms       INTEGER NOT NULL,
    steps       INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS subscriptions (
    account_id  INTEGER NOT NULL,
    ts_ms       INTEGER NOT NULL,
    active      INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS snapshots (
    account_id  INTEGER PRIMARY KEY,
    state_json  TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS meta (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);
";

const INDEX_SQL: &str = "
CREATE INDEX IF NOT EXISTS ix_jar_events_acct ON jar_events (account_id, ts_ms, seq);
CREATE INDEX IF NOT EXISTS ix_step_packages_acct ON step_packages (account_id, ts_ms);
CREATE INDEX IF NOT EXISTS ix_boosted_step_packages_acct ON boosted_step_packages (account_id, ts_ms);
CREATE INDEX IF NOT EXISTS ix_subscriptions_acct ON subscriptions (account_id, ts_ms);
";

/// Create all six tables if they do not already exist.
pub fn init_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(SCHEMA_SQL).context("init_schema")
}

/// Create the per-account indexes. Run after bulk load.
pub fn create_indexes(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(INDEX_SQL).context("create_indexes")
}
