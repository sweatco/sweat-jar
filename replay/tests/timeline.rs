//! `timeline::load_user` against a fixture DuckDB built by `build_db`.

mod common;
use common::write_fixture_dataset;
use replay::db;
use replay::db::ingest::{build_db, BuildOpts};
use replay::timeline::load_user;
use sweat_jar::replay::engine::Action;

fn conn(dir: &std::path::Path) -> duckdb::Connection {
    let src = dir.join("src");
    write_fixture_dataset(&src);
    let dbp = dir.join("t.duckdb");
    let mut c = db::open_write(&dbp).unwrap();
    db::schema::init_schema(&c).unwrap();
    build_db(&mut c, &BuildOpts { source_dir: &src, accounts: None, sample: None }).unwrap();
    c
}

#[test]
fn account_200_maps_all_event_kinds() {
    let d = tempfile::tempdir().unwrap();
    let c = conn(d.path());
    let (slice, tl) = load_user(&c, 200).unwrap();
    assert_eq!(slice.near_account_id, "near200");
    assert!(!slice.existed_at_start);
    assert_eq!(slice.timezone_ms, Some(-18_000_000));
    assert_eq!(slice.onchain_claimed, 123);

    let kinds: Vec<&str> = tl
        .events
        .iter()
        .map(|e| match &e.action {
            Action::Deposit { .. } => "deposit",
            Action::RecordScore(_) => "score",
            Action::ApplyBooster { .. } => "booster",
            Action::Claim => "claim",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, vec!["deposit", "score", "booster", "claim"]);
}

#[test]
fn account_300_withdraw_all_and_empty_score_skipped() {
    let d = tempfile::tempdir().unwrap();
    let c = conn(d.path());
    let (_slice, tl) = load_user(&c, 300).unwrap();
    // record_score '[]' produces no event; deposit + withdraw_all remain.
    assert_eq!(tl.events.len(), 2);
    assert!(matches!(tl.events[1].action, Action::WithdrawAll { ref product_ids }
        if product_ids == &vec!["365d_12apy".to_string()]));
}

#[test]
fn unknown_account_is_err() {
    let d = tempfile::tempdir().unwrap();
    let c = conn(d.path());
    assert!(load_user(&c, 999).is_err());
}
