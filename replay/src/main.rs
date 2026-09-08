use std::collections::HashSet;

use anyhow::Context;
use clap::Parser;

use replay::{cli, db};

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    match cli.cmd {
        cli::Cmd::FetchProducts { .. } => anyhow::bail!("unimplemented: fetch-products"),
        cli::Cmd::BuildDb {
            db: db_path,
            test_data_dir,
            only,
            accounts,
            sample,
        } => {
            let accounts: Option<HashSet<i64>> = match accounts {
                Some(path) => Some(read_accounts(&path)?),
                None => None,
            };
            let mut conn = db::open_write(&db_path)?;
            db::schema::init_schema(&conn)?;
            let counts = db::ingest::build_db(
                &mut conn,
                &db::ingest::BuildOpts {
                    test_data_dir: &test_data_dir,
                    only: &only,
                    accounts: accounts.as_ref(),
                    sample,
                },
            )?;
            for (table, n) in counts {
                println!("{table}: {n}");
            }
            Ok(())
        }
        cli::Cmd::Run { .. } => anyhow::bail!("unimplemented: run"),
    }
}

/// One `account_id` per line; blank lines and `#` comments ignored.
fn read_accounts(path: &std::path::Path) -> anyhow::Result<HashSet<i64>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read accounts file {}", path.display()))?;
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.parse::<i64>().with_context(|| format!("parse account_id {l:?}")))
        .collect()
}
