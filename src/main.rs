use clap::Parser;
use cxc::cli::{self, Cli};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();
    cli::run_cli(cli).await?;
    Ok(())
}
