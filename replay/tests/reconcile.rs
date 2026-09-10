//! `reconcile_user` over a DuckDB fixture built from the parquet dataset.

mod common;
use common::write_fixture_dataset;
use replay::db;
use replay::db::ingest::{build_db, BuildOpts};
use replay::products::load_products;
use replay::reconcile::reconcile_user;
use replay::snapshot::DbSnapshotSource;

fn build_fixture_db(dir: &std::path::Path) -> std::path::PathBuf {
    let src = dir.join("src");
    write_fixture_dataset(&src);
    let dbp = dir.join("r.duckdb");
    let mut c = db::open_write(&dbp).unwrap();
    db::schema::init_schema(&c).unwrap();
    build_db(&mut c, &BuildOpts { source_dir: &src, accounts: None, sample: None }).unwrap();
    drop(c);
    dbp
}

fn products() -> Vec<sweat_jar_model::data::product::Product> {
    load_products(std::path::Path::new("tests/fixtures/products.json")).unwrap()
}

#[test]
fn fresh_account_reconciles_without_baseline_flag() {
    let d = tempfile::tempdir().unwrap();
    let dbp = build_fixture_db(d.path());
    let c = db::open_read(&dbp).unwrap();
    let snap = DbSnapshotSource::new(&dbp);

    // account 300: fresh (existed_at_start = false), deposit -> withdraw_all.
    let row = reconcile_user(&c, 300, &products(), &snap).unwrap();
    assert_eq!(row.status, "ok", "row: {row:?}");
}

#[test]
fn existed_at_start_account_without_snapshot_is_no_baseline() {
    let d = tempfile::tempdir().unwrap();
    let dbp = build_fixture_db(d.path());
    let c = db::open_read(&dbp).unwrap();
    let snap = DbSnapshotSource::new(&dbp);

    // account 100: existed_at_start = true, no events, no snapshot row.
    let row = reconcile_user(&c, 100, &products(), &snap).unwrap();
    assert_eq!(row.status, "no_baseline", "row: {row:?}");
    assert_eq!(row.n_claims, 0, "row: {row:?}");
}

#[test]
fn fresh_account_with_claim_reconciles_ok() {
    let d = tempfile::tempdir().unwrap();
    let dbp = build_fixture_db(d.path());
    let c = db::open_read(&dbp).unwrap();
    let snap = DbSnapshotSource::new(&dbp);

    // account 200: fresh, tiered-score deposit + record_score + booster + claim(123).
    let row = reconcile_user(&c, 200, &products(), &snap).unwrap();
    assert_eq!(row.status, "ok", "row: {row:?}");
    assert!(row.n_claims >= 1, "row: {row:?}");
    assert_eq!(row.actual_total_claim, "123", "row: {row:?}");
}
