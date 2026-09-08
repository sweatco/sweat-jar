use replay::db;
use replay::db::ingest::{build_db, BuildOpts};

#[test]
fn ingest_users_and_subscriptions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: std::path::Path::new("tests/fixtures"),
            only: &["users".into(), "subscriptions".into()],
            accounts: None,
            sample: None,
        },
    )
    .unwrap();

    let users: i64 = conn
        .query_row("SELECT count(*) FROM users", [], |r| r.get(0))
        .unwrap();
    assert_eq!(users, 2);
    let near: String = conn
        .query_row("SELECT near_account_id FROM users WHERE account_id=4", [], |r| r.get(0))
        .unwrap();
    assert!(near.starts_with("0c1570"));
    let subs: Vec<(i64, i64, i64)> = conn
        .prepare("SELECT account_id, ts_ms, active FROM subscriptions ORDER BY ts_ms")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(subs.len(), 3);
    assert_eq!(subs[0], (4, 1_766_133_726_000, 1));
    assert_eq!(subs.iter().filter(|(_, _, a)| *a == 0).count(), 1);
}

#[test]
fn build_db_sample_limits_to_first_user() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: std::path::Path::new("tests/fixtures"),
            only: &["users".into(), "subscriptions".into()],
            accounts: None,
            sample: Some(1),
        },
    )
    .unwrap();

    let user_ids: Vec<i64> = conn
        .prepare("SELECT account_id FROM users ORDER BY account_id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(user_ids, vec![4]);

    let sub_accts: Vec<i64> = conn
        .prepare("SELECT account_id FROM subscriptions")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(sub_accts.len(), 2);
    assert!(sub_accts.iter().all(|&a| a == 4));
}

#[test]
fn ingest_jar_events_filters_window_and_merge() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: std::path::Path::new("tests/fixtures"),
            only: &["users".into(), "jar_events".into()],
            accounts: None,
            sample: None,
        },
    )
    .unwrap();

    let rows: Vec<(i64, String, String, i64)> = conn
        .prepare("SELECT account_id,event_type,amount,ts_ms FROM jar_events ORDER BY ts_ms,seq")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].1, "deposit");
    assert_eq!(rows[0].2, "15760000000000000000");
    let claim = rows.iter().find(|r| r.1 == "claim").unwrap();
    assert_eq!(claim.2, "81505017066108257");
    assert_eq!(rows.iter().filter(|r| r.1 == "withdraw" && r.0 == 4).count(), 1);
    assert!(rows.iter().all(|r| r.1 != "merge"));
    assert!(rows.iter().all(|r| r.2 != "999"));
    assert!(rows.iter().all(|r| r.3 != 1_774_017_710_100));
}

#[test]
fn ingest_jar_events_respects_keep() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: std::path::Path::new("tests/fixtures"),
            only: &["users".into(), "jar_events".into()],
            accounts: None,
            sample: Some(1),
        },
    )
    .unwrap();

    let rows: Vec<(i64, String)> = conn
        .prepare("SELECT account_id,event_type FROM jar_events")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (4, "withdraw".to_string()));
}

#[test]
fn ingest_jar_events_rejects_unknown_event_type() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let mut f = std::fs::File::create(dir.path().join("jar_events.csv")).unwrap();
    writeln!(
        f,
        "account_id,jar_id,product_id,product_name,near_block_timestamp,event_type,amount,fee_amount,deposit_ids"
    )
    .unwrap();
    writeln!(
        f,
        "4,7,90d_3apy,The Starter,2026-05-01T00:00:00.000Z,frobnicate,5,0,9"
    )
    .unwrap();
    drop(f);

    let path = dir.path().join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    let err = build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: dir.path(),
            only: &["jar_events".into()],
            accounts: None,
            sample: None,
        },
    );
    assert!(err.is_err());
}

#[test]
fn ingest_step_packages_clamps_and_windows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: std::path::Path::new("tests/fixtures"),
            only: &["users".into(), "step_packages".into()],
            accounts: None,
            sample: None,
        },
    )
    .unwrap();

    let rows: Vec<(i64, i64, i64)> = conn
        .prepare("SELECT account_id,ts_ms,steps FROM step_packages ORDER BY ts_ms")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|r| r.2 == 65535));
    assert!(rows.iter().any(|r| r.0 == 4 && r.2 == 7000));
    assert!(rows.iter().all(|r| r.1 != 1_774_017_710_000));
    let sept5 = replay::parse::space_utc_to_epoch_ms("2026-09-05 00:00:00 UTC").unwrap() as i64;
    assert!(rows.iter().all(|r| r.1 != sept5));
}

#[test]
fn ingest_step_packages_batch_seam() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let mut f = std::fs::File::create(dir.path().join("step_packages.csv")).unwrap();
    writeln!(f, "account_id,created_at,steps").unwrap();
    for (i, mm) in [10, 11, 12, 13, 14].iter().enumerate() {
        // all in-window (2026-04), distinct timestamps
        writeln!(f, "4,2026-04-01 00:00:{mm} UTC,{}", 100 + i).unwrap();
    }
    drop(f);

    let path = dir.path().join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    std::env::set_var("REPLAY_STEP_BATCH", "2");
    let r = build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: dir.path(),
            only: &["step_packages".into()],
            accounts: None,
            sample: None,
        },
    );
    std::env::remove_var("REPLAY_STEP_BATCH");
    r.unwrap();

    let rows: Vec<(i64, i64)> = conn
        .prepare("SELECT ts_ms,steps FROM step_packages ORDER BY ts_ms")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows.iter().map(|r| r.1).collect::<Vec<_>>(), vec![100, 101, 102, 103, 104]);
}

#[test]
fn ingest_step_packages_negative_steps_errs() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let mut f = std::fs::File::create(dir.path().join("step_packages.csv")).unwrap();
    writeln!(f, "account_id,created_at,steps").unwrap();
    writeln!(f, "4,2026-05-02 12:00:00 UTC,-5").unwrap();
    drop(f);

    let path = dir.path().join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    let err = build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: dir.path(),
            only: &["step_packages".into()],
            accounts: None,
            sample: None,
        },
    );
    assert!(err.is_err());
}

#[test]
fn ingest_snapshots_joins_on_near_account_id() {
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

    let cnt: i64 = conn
        .query_row("SELECT count(*) FROM snapshots WHERE account_id=36988193", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cnt, 1);
    let json: String = conn
        .query_row("SELECT state_json FROM snapshots WHERE account_id=36988193", [], |r| r.get(0))
        .unwrap();
    assert!(json.starts_with("{\"near_account_id\":\"9b6b8403"));
}

#[test]
fn ingest_snapshots_missing_file_is_noop() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: dir.path(),
            only: &["snapshots".into()],
            accounts: None,
            sample: None,
        },
    )
    .unwrap();
    let cnt: i64 = conn
        .query_row("SELECT count(*) FROM snapshots", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cnt, 0);
}

#[test]
fn build_db_rejects_unknown_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    let err = build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: std::path::Path::new("tests/fixtures"),
            only: &["users".into(), "bogus".into()],
            accounts: None,
            sample: None,
        },
    );
    assert!(err.is_err());
}

#[test]
fn build_db_writes_meta() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: std::path::Path::new("tests/fixtures"),
            only: &[],
            accounts: None,
            sample: None,
        },
    )
    .unwrap();
    let v: String = conn
        .query_row("SELECT value FROM meta WHERE key='window_h_ms'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, "1774017710156");
}

#[test]
fn schema_creates_expected_tables() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    let mut names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    names.retain(|n| !n.starts_with("sqlite_"));
    assert_eq!(
        names,
        vec![
            "jar_events",
            "meta",
            "snapshots",
            "step_packages",
            "subscriptions",
            "users"
        ]
    );
}

#[test]
fn open_read_missing_path_errs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does-not-exist.db");
    assert!(db::open_read(&path).is_err());
}

#[test]
fn create_indexes_after_init_schema_registers_ix() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    db::schema::create_indexes(&conn).unwrap();
    let mut names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'ix_%' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    names.sort();
    assert_eq!(
        names,
        vec![
            "ix_jar_events_acct",
            "ix_step_packages_acct",
            "ix_subscriptions_acct"
        ]
    );
}
