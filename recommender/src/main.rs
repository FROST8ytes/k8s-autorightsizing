use clap::Parser;
use recommender::{Cli, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        println!("AWS Managed Prometheus URL: {}", cli.amp_url);
        println!("AWS Region: {}", cli.region);
    }

    println!("Recommender running...");

    Ok(())
}
