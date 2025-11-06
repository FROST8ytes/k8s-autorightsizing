use clap::Parser;
use log::{debug, info};
use recommender::{Cli, Result};

fn init_logger(verbose: bool) {
    let log_level = if verbose {
        "debug"
    } else {
        "info"
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .format_timestamp_secs()
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    init_logger(cli.verbose);

    info!("Starting Kubernetes Resource Recommender");
    debug!("AWS Managed Prometheus URL: {}", cli.amp_url);
    debug!("AWS Region: {}", cli.region);

    // TODO: Implement recommendation logic
    info!("Recommendation analysis not yet implemented");

    Ok(())
}
