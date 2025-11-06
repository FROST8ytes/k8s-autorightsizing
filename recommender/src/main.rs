use clap::Parser;
use log::{debug, info};
use recommender::{Cli, Config, Result, lib::kubernetes::KubernetesLoader};

fn init_logger(verbose: bool) {
    let log_level = if verbose { "debug" } else { "info" };

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

    let config = Config::new(
        String::from(cli.amp_url),
        cli.region.to_string(),
        cli.context,
        cli.namespace,
    );

    let k8s_loader = KubernetesLoader::new(config.clone()).await?;
    let workloads = k8s_loader.get_deployments().await?;

    info!("Workloads found: {workloads:?}");

    info!("Recommendation analysis not yet implemented");

    Ok(())
}
