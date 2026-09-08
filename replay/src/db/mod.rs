//! SQLite connection helpers for the replay database.

pub mod schema;

use std::path::Path;

use anyhow::Context;
use rusqlite::{Connection, OpenFlags};

/// Open (creating if needed) for bulk writing, with pragmas tuned for a
/// one-shot build: `synchronous = OFF`, `journal_mode = MEMORY`.
pub fn open_write(path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(path)
        .with_context(|| format!("open_write: {}", path.display()))?;
    conn.execute_batch("PRAGMA synchronous = OFF; PRAGMA journal_mode = MEMORY;")
        .context("open_write: set build pragmas")?;
    Ok(conn)
}

/// Open an existing database read-only.
pub fn open_read(path: &Path) -> anyhow::Result<Connection> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("open_read: {}", path.display()))
}
