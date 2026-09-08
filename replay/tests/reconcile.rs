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

    // 36988193 has a fixture snapshot + a deposit into 365d_12apy and one claim.
    // Signature verification is disabled (public_key stripped) so the deposit
    // lands, and the contract's FT gates now cover `feature="replay-engine"`, so
    // `claim_total` returns a synchronous value: the replay reconciles to a real
    // non-zero accrued-interest figure with exactly one recorded claim.
    assert_eq!(row.status, "ok");
    assert_ne!(row.calculated_total_claim, "0");
    assert_eq!(row.n_claims, 1);
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
fn reconcile_user_archival_unreachable_is_error_row() {
    let d = tempfile::tempdir().unwrap();
    let (conn, _path) = fixture_db(d.path());
    // Unroutable endpoint: raw_account returns Err quickly; reconcile_user must
    // turn that into an `error:` row, not propagate it.
    let snap = ArchivalRpcSnapshotSource {
        rpc_url: "http://127.0.0.1:1/".into(),
        jar_contract: "v2.jars.sweat".into(),
        block_height: replay::parse::H_BLOCK,
    };
    let row = reconcile_user(&conn, 36988193, &products(), &snap).unwrap();
    assert!(row.status.starts_with("error:"), "status {}", row.status);
}

#[test]
fn reconcile_user_unknown_account_errs() {
    let d = tempfile::tempdir().unwrap();
    let (conn, path) = fixture_db(d.path());
    let snap = DbSnapshotSource::new(&path);
    assert!(reconcile_user(&conn, 999999, &products(), &snap).is_err());
}
