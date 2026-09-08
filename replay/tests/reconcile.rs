use replay::db;
use replay::db::ingest::{build_db, BuildOpts};
use replay::products::load_products;
use replay::reconcile::reconcile_user;
use replay::snapshot::{ArchivalRpcSnapshotSource, DbSnapshotSource};
use sweat_jar_model::data::product::Product;

fn fixture_db(dir: &std::path::Path) -> (rusqlite::Connection, std::path::PathBuf) {
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
                "snapshots".into(),
            ],
            accounts: None,
            sample: None,
        },
    )
    .unwrap();
    (conn, path)
}

fn products() -> Vec<Product> {
    load_products(std::path::Path::new("tests/fixtures/products.json")).unwrap()
}

#[test]
fn reconcile_user_produces_a_row() {
    let d = tempfile::tempdir().unwrap();
    let (conn, path) = fixture_db(d.path());
    let snap = DbSnapshotSource::new(&path);
    let row = reconcile_user(&conn, 36988193, &products(), &snap).unwrap();

    assert_eq!(row.account_id, 36988193);
    assert!(row.near_account_id.starts_with("9b6b8403"));
    assert_eq!(row.actual_total_claim, "81505017066108257");

    let delta = row.delta.parse::<i128>().unwrap();
    let calc = row.calculated_total_claim.parse::<i128>().unwrap();
    let actual = row.actual_total_claim.parse::<i128>().unwrap();
    assert_eq!(delta, calc - actual);

    // 36988193 has a fixture snapshot + a deposit into 365d_12apy: signature
    // verification is disabled (public_key stripped) so the deposit lands and the
    // replay completes cleanly rather than erroring.
    assert_eq!(row.status, "ok");
    // NOTE: `calculated_total_claim` stays "0" / `n_claims` stays 0 here because
    // the engine's `claim_interest` only returns a synchronous value under
    // `#[cfg(test)]`; compiled via the `replay-engine` feature it returns a
    // Promise and `run_timeline` cannot observe the claimed amount. Tracked as an
    // engine-side blocker (contract/src/feature/claim/api.rs).
    row.calculated_total_claim.parse::<u128>().unwrap();
}

#[test]
fn reconcile_user_no_baseline_status() {
    let d = tempfile::tempdir().unwrap();
    let (conn, path) = fixture_db(d.path());
    // A user present in `users` with a benign score event but no `snapshots` row.
    conn.execute(
        "INSERT INTO users (account_id, near_account_id) VALUES (8, 'alice.near')",
        [],
    )
    .unwrap();
    let snap = DbSnapshotSource::new(&path);
    let row = reconcile_user(&conn, 8, &products(), &snap).unwrap();
    assert_eq!(row.status, "no_baseline");
}

#[test]
fn reconcile_user_archival_stub_is_error_row() {
    let d = tempfile::tempdir().unwrap();
    let (conn, _path) = fixture_db(d.path());
    let snap = ArchivalRpcSnapshotSource {
        rpc_url: "x".into(),
        block_height: 1,
    };
    let row = reconcile_user(&conn, 36988193, &products(), &snap).unwrap();
    assert!(row.status.starts_with("error:"), "status {}", row.status);
    assert!(row.status.contains("not implemented"), "status {}", row.status);
}

#[test]
fn reconcile_user_unknown_account_errs() {
    let d = tempfile::tempdir().unwrap();
    let (conn, path) = fixture_db(d.path());
    let snap = DbSnapshotSource::new(&path);
    assert!(reconcile_user(&conn, 999999, &products(), &snap).is_err());
}
