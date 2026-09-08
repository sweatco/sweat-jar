use replay::db;

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
