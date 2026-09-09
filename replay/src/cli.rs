use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(clap::Subcommand)]
pub enum Cmd {
    FetchProducts {
        #[arg(long, default_value = "test_data/products.json")]
        out: PathBuf,
    },
    BuildDb {
        #[arg(long)]
        db: PathBuf,
        #[arg(long, default_value = "test_data")]
        test_data_dir: PathBuf,
        #[arg(long, value_delimiter = ',')]
        only: Vec<String>,
        #[arg(long)]
        accounts: Option<PathBuf>,
        #[arg(long)]
        sample: Option<usize>,
    },
    Run {
        #[arg(long)]
        db: PathBuf,
        #[arg(long, default_value = "reconciliation.csv")]
        out: PathBuf,
        #[arg(long, default_value = "test_data/products.json")]
        products: PathBuf,
        #[arg(long)]
        threads: Option<usize>,
        #[arg(long)]
        shard: Option<String>,
        #[arg(long)]
        accounts: Option<PathBuf>,
        #[arg(long)]
        sample: Option<usize>,
        #[arg(long, default_value_t = 1e-6)]
        tolerance: f64,
        /// Fetch each account's block-H state live from a NEAR archival node
        /// (jar contract `get_account` view) instead of the local `snapshots`
        /// table. Slow (~1 RPC per account) but needs no pre-populated snapshots.
        #[arg(long)]
        archival: bool,
        /// Archival JSON-RPC endpoint used when `--archival` is set.
        #[arg(long, default_value = crate::snapshot::FASTNEAR_ARCHIVAL_RPC)]
        archival_rpc_url: String,
    },
    /// Replay one account and print its per-claim breakdown vs the on-chain
    /// claim amounts (to trace where a non-zero `delta` comes from).
    Explain {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        account: i64,
        #[arg(long, default_value = "test_data/products.json")]
        products: PathBuf,
        #[arg(long)]
        archival: bool,
        #[arg(long, default_value = crate::snapshot::FASTNEAR_ARCHIVAL_RPC)]
        archival_rpc_url: String,
    },
}
