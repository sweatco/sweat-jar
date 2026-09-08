use clap::Parser;

use replay::cli;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    match cli.cmd {
        cli::Cmd::FetchProducts { .. } => anyhow::bail!("unimplemented: fetch-products"),
        cli::Cmd::BuildDb { .. } => anyhow::bail!("unimplemented: build-db"),
        cli::Cmd::Run { .. } => anyhow::bail!("unimplemented: run"),
    }
}
