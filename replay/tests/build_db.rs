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
