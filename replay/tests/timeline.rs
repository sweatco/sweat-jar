use replay::db;
use replay::db::ingest::{build_db, BuildOpts};
use replay::timeline::load_user;
use sweat_jar::replay::engine::Action;

fn fixture_db(dir: &std::path::Path) -> rusqlite::Connection {
    let path = dir.join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: std::path::Path::new("tests/fixtures"),
            only: &[
                "users".into(),
                "jar_events".into(),
                "step_packages".into(),
                "subscriptions".into(),
            ],
            accounts: None,
            sample: None,
        },
    )
    .unwrap();
    conn
}

#[test]
fn load_user_builds_sorted_timeline() {
    let d = tempfile::tempdir().unwrap();
    let conn = fixture_db(d.path());
    let (slice, timeline) = load_user(&conn, 36988193).unwrap();

    assert_eq!(slice.onchain_claimed, 81_505_017_066_108_257);
    assert!(slice.near_account_id.starts_with("9b6b8403"));

    let claims = timeline
        .events
        .iter()
        .filter(|e| matches!(e.action, Action::Claim))
        .count();
    assert_eq!(claims, 1);

    assert!(timeline.events.windows(2).all(|w| (
        w[0].ts_ms,
        w[0].action.rank(),
        w[0].seq
    ) <= (w[1].ts_ms, w[1].action.rank(), w[1].seq)));

    assert!(timeline
        .events
        .iter()
        .any(|e| matches!(e.action, Action::Deposit { amount: 15_760_000_000_000_000_000, .. })));
    assert!(timeline
        .events
        .iter()
        .any(|e| matches!(e.action, Action::RecordScore(65535))));
    assert!(timeline
        .events
        .iter()
        .any(|e| matches!(e.action, Action::SetIncreasedScoreCap(true))));
}

#[test]
fn load_user_collapses_same_ts_claims() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("t.db");
    let conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    conn.execute(
        "INSERT INTO users (account_id, near_account_id) VALUES (7, 'deadbeef')",
        [],
    )
    .unwrap();
    // two claim rows at the same ts_ms, one at a later ts
    conn.execute_batch(
        "INSERT INTO jar_events (account_id, ts_ms, seq, event_type, product_id, amount) VALUES
           (7, 1000, 5, 'claim', 'p', '100'),
           (7, 1000, 4, 'claim', 'p', '250'),
           (7, 2000, 9, 'claim', 'p', '7');",
    )
    .unwrap();

    let (slice, timeline) = load_user(&conn, 7).unwrap();
    let claims: Vec<_> = timeline
        .events
        .iter()
        .filter(|e| matches!(e.action, Action::Claim))
        .collect();
    assert_eq!(claims.len(), 2, "one Claim per distinct ts_ms");
    // collapsed claim at ts 1000 keeps the MIN seq (4)
    assert_eq!(claims.iter().find(|e| e.ts_ms == 1000).unwrap().seq, 4);
    assert_eq!(slice.onchain_claimed, 357);
}

#[test]
fn load_user_unknown_account_errs() {
    let d = tempfile::tempdir().unwrap();
    let conn = fixture_db(d.path());
    assert!(load_user(&conn, 424242).is_err());
}
