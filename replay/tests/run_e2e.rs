mod common;
use common::write_fixture_dataset;

use std::path::{Path, PathBuf};

use replay::db;
use replay::db::ingest::{build_db, BuildOpts};
use replay::run::{parse_shard, run, RunOpts};

const PRODUCTS: &str = "tests/fixtures/products.json";

fn fixture_db(dir: &Path) -> PathBuf {
    let src = dir.join("src");
    write_fixture_dataset(&src);
    let path = dir.join("e2e.duckdb");
    let mut conn = db::open_write(&path).unwrap();
    db::schema::init_schema(&conn).unwrap();
    build_db(&mut conn, &BuildOpts { source_dir: &src, accounts: None, sample: None }).unwrap();
    drop(conn);
    path
}

fn opts(db: PathBuf, out: PathBuf) -> RunOpts {
    RunOpts {
        db,
        out,
        products: PRODUCTS.into(),
        threads: 2,
        shard: None,
        accounts: None,
        sample: None,
        tolerance: 1e-6,
        archival_rpc_url: None,
    }
}

fn sorted_rows(csv: &str) -> Vec<String> {
    let mut lines: Vec<String> = csv.lines().skip(1).map(str::to_string).collect();
    lines.sort();
    lines
}

#[test]
fn run_end_to_end_writes_csv() {
    let d = tempfile::tempdir().unwrap();
    let db = fixture_db(d.path());
    let out = d.path().join("rec.csv");
    let summary = run(&opts(db, out.clone())).unwrap();

    assert_eq!(summary.processed, 3);
    assert_eq!(summary.ok, 2);
    assert_eq!(summary.no_baseline, 1);

    let body = std::fs::read_to_string(&out).unwrap();
    assert_eq!(
        body.lines().next().unwrap(),
        "account_id,near_account_id,calculated_total_claim,actual_total_claim,delta,rel_delta,n_claims,status"
    );
    assert_eq!(body.lines().count(), 4);

    let row = body
        .lines()
        .find(|l| l.starts_with("200,"))
        .expect("row for 200");
    let cols: Vec<&str> = row.split(',').collect();
    assert_eq!(cols[7], "ok");
    assert_eq!(cols[3], "123");
}

#[test]
fn run_threads_1_is_deterministic() {
    let d = tempfile::tempdir().unwrap();
    let db = fixture_db(d.path());

    let out1 = d.path().join("a.csv");
    let mut o1 = opts(db.clone(), out1.clone());
    o1.threads = 1;
    run(&o1).unwrap();

    let out2 = d.path().join("b.csv");
    let mut o2 = opts(db, out2.clone());
    o2.threads = 1;
    run(&o2).unwrap();

    let a = std::fs::read_to_string(&out1).unwrap();
    let b = std::fs::read_to_string(&out2).unwrap();
    assert_eq!(sorted_rows(&a), sorted_rows(&b));
    assert_eq!(a, b);
}

#[test]
fn run_shard_splits_worklist() {
    let d = tempfile::tempdir().unwrap();
    let db = fixture_db(d.path());

    let out0 = d.path().join("s0.csv");
    let mut o0 = opts(db.clone(), out0.clone());
    o0.shard = Some((0, 2));
    let s0 = run(&o0).unwrap();

    let out1 = d.path().join("s1.csv");
    let mut o1 = opts(db, out1.clone());
    o1.shard = Some((1, 2));
    let s1 = run(&o1).unwrap();

    assert_eq!(s0.processed + s1.processed, 3);

    let ids = |p: &Path| -> Vec<String> {
        std::fs::read_to_string(p)
            .unwrap()
            .lines()
            .skip(1)
            .map(|l| l.split(',').next().unwrap().to_string())
            .collect()
    };
    for id in ids(&out0) {
        assert!(!ids(&out1).contains(&id), "{id} in both shards");
    }
}

#[test]
fn run_sample_limits() {
    let d = tempfile::tempdir().unwrap();
    let db = fixture_db(d.path());
    let out = d.path().join("rec.csv");
    let mut o = opts(db, out);
    o.sample = Some(1);
    assert_eq!(run(&o).unwrap().processed, 1);
}

#[test]
fn parse_shard_validates() {
    assert_eq!(parse_shard("1/4").unwrap(), (1, 4));
    assert!(parse_shard("4/4").is_err());
    assert!(parse_shard("0/0").is_err());
    assert!(parse_shard("x").is_err());
    assert!(parse_shard("1/2/3").is_err());
}
