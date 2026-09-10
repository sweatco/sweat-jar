use replay::db;

#[test]
fn open_write_then_read_roundtrips_schema() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("t.duckdb");
    {
        let conn = db::open_write(&path).unwrap();
        db::schema::init_schema(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO accounts VALUES (1, 'near1', true, 10800000);"
        ).unwrap();
    }
    let conn = db::open_read(&path).unwrap();
    let n: i64 = conn
        .query_row("SELECT count(*) FROM accounts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
}
