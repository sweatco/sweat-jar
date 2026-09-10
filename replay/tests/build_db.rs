mod common;
use common::write_fixture_dataset;
use replay::db;
use replay::db::ingest::{build_db, BuildOpts};

fn built(dir: &std::path::Path) -> duckdb::Connection {
    let src = dir.join("src");
    write_fixture_dataset(&src);
    let dbp = dir.join("out.duckdb");
    let mut conn = db::open_write(&dbp).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(&mut conn, &BuildOpts { source_dir: &src, accounts: None, sample: None }).unwrap();
    conn
}

#[test]
fn events_are_success_only_and_sorted() {
    let d = tempfile::tempdir().unwrap();
    let conn = built(d.path());
    let n: i64 = conn.query_row("SELECT count(*) FROM events", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 7); // all fixture rows are SUCCESS_VALUE
    let ordered: bool = conn
        .query_row(
            "SELECT bool_and(ok) FROM (SELECT (backend_account_id, ts_ms, log_index) >= \
             lag((backend_account_id, ts_ms, log_index)) OVER () AS ok FROM events)",
            [],
            |r| r.get::<_, Option<bool>>(0).map(|o| o.unwrap_or(true)),
        )
        .unwrap();
    assert!(ordered);
}

#[test]
fn accounts_join_carries_timezone_and_existed_flag() {
    let d = tempfile::tempdir().unwrap();
    let conn = built(d.path());
    let (existed, tz): (bool, Option<i64>) = conn
        .query_row(
            "SELECT existed_at_start, timezone_ms FROM accounts WHERE backend_account_id = 100",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(existed);
    assert_eq!(tz, Some(10_800_000));
    let tz300: Option<i64> = conn
        .query_row(
            "SELECT timezone_ms FROM accounts WHERE backend_account_id = 300",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tz300, None);
}

#[test]
fn sample_limits_accounts_and_their_events() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("src");
    write_fixture_dataset(&src);
    let dbp = d.path().join("s.duckdb");
    let mut conn = db::open_write(&dbp).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(&mut conn, &BuildOpts { source_dir: &src, accounts: None, sample: Some(1) }).unwrap();
    let a: i64 = conn.query_row("SELECT count(*) FROM accounts", [], |r| r.get(0)).unwrap();
    assert_eq!(a, 1);
    let stray: i64 = conn
        .query_row(
            "SELECT count(*) FROM events WHERE backend_account_id NOT IN (SELECT backend_account_id FROM accounts)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stray, 0);
}
