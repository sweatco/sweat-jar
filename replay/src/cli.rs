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
    },
}
