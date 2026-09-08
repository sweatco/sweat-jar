//! Regression guard: near-sdk's `testing_env!` persists thread-local mock
//! storage across calls on one thread. `engine::run_timeline` / `Context::new`
//! must fully reset that storage per user, so a second user reconciled on the
//! same worker thread never inherits the first user's contract state.

use std::io::Write;
use std::path::{Path, PathBuf};

use replay::db;
use replay::db::ingest::{build_db, BuildOpts};
use replay::products::load_products;
use replay::reconcile::{reconcile_user, ReconRow};
use replay::run::{run, RunOpts};
use replay::snapshot::DbSnapshotSource;
use sweat_jar_model::data::product::Product;

// Account A: jar deposit + claim + baseline snapshot -> non-zero calculated claim.
const ACCOUNT_A: i64 = 36988193;
// Account B: present in users.csv, but no jar_events / step_packages /
// subscriptions / snapshot -> a completely empty user.
const ACCOUNT_B: i64 = 999;

fn fixture_db(dir: &Path) -> (rusqlite::Connection, PathBuf) {
    let path = dir.join("t.db");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(
        &mut conn,
        &BuildOpts {
            test_data_dir: Path::new("tests/fixtures"),
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
    load_products(Path::new("tests/fixtures/products.json")).unwrap()
}

fn find_row<'a>(csv: &'a str, account_id: i64) -> Vec<&'a str> {
    let prefix = format!("{account_id},");
    csv.lines()
        .find(|l| l.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no CSV row for {account_id}"))
        .split(',')
        .collect()
}

/// Through the threaded driver with `threads: 1`: A and B land on the same
/// worker, processed back-to-back. B must not inherit A's mock storage.
#[test]
fn second_user_on_a_worker_is_not_polluted_by_the_first() {
    let d = tempfile::tempdir().unwrap();
    let (conn, path) = fixture_db(d.path());
    drop(conn);

    let accounts = d.path().join("accounts.txt");
    writeln!(std::fs::File::create(&accounts).unwrap(), "{ACCOUNT_A}\n{ACCOUNT_B}").unwrap();

    let out = d.path().join("rec.csv");
    let summary = run(&RunOpts {
        db: path,
        out: out.clone(),
        products: "tests/fixtures/products.json".into(),
        threads: 1,
        shard: None,
        accounts: Some(accounts),
        sample: None,
        tolerance: 1e-6,
    })
    .unwrap();
    assert_eq!(summary.processed, 2);

    let body = std::fs::read_to_string(&out).unwrap();

    let a = find_row(&body, ACCOUNT_A);
    assert_eq!(a[7], "ok", "account A status");
    assert_ne!(a[2], "0", "account A calculated_total_claim");
    let a_calc = a[2].to_string();

    let b = find_row(&body, ACCOUNT_B);
    assert_eq!(b[2], "0", "account B calculated_total_claim must be zero");
    assert_eq!(b[3], "0", "account B actual_total_claim");
    assert_eq!(b[6], "0", "account B n_claims");
    assert!(
        b[7] == "no_baseline" || b[7] == "ok",
        "account B status was {:?}",
        b[7]
    );
    assert!(!b[7].starts_with("error:"), "account B status was {:?}", b[7]);
    assert_ne!(b[2], a_calc, "account B inherited account A's calculated claim");
}

fn assert_empty(row: &ReconRow) {
    assert_eq!(row.calculated_total_claim, "0", "B calculated");
    assert_eq!(row.actual_total_claim, "0", "B actual");
    assert_eq!(row.n_claims, 0, "B n_claims");
    assert!(!row.status.starts_with("error:"), "B status {:?}", row.status);
}

/// Direct variant, no threading harness: call `reconcile_user` twice in sequence
/// on this (named) test thread. B's result is independent of whether A ran first.
#[test]
fn reconcile_user_back_to_back_does_not_leak_storage() {
    let d = tempfile::tempdir().unwrap();
    let (conn, path) = fixture_db(d.path());
    let snap = DbSnapshotSource::new(&path);
    let products = products();

    // A then B
    let a1 = reconcile_user(&conn, ACCOUNT_A, &products, &snap).unwrap();
    assert_eq!(a1.status, "ok");
    assert_ne!(a1.calculated_total_claim, "0");
    let b1 = reconcile_user(&conn, ACCOUNT_B, &products, &snap).unwrap();
    assert_empty(&b1);

    // B then A: B is still empty; A still reconciles to the same non-zero figure.
    let b2 = reconcile_user(&conn, ACCOUNT_B, &products, &snap).unwrap();
    assert_empty(&b2);
    let a2 = reconcile_user(&conn, ACCOUNT_A, &products, &snap).unwrap();
    assert_eq!(a2.status, "ok");
    assert_eq!(a2.calculated_total_claim, a1.calculated_total_claim);

    assert_eq!(b1.calculated_total_claim, b2.calculated_total_claim);
}
