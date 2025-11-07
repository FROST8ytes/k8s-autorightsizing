use clap::Parser;
use log::{debug, info};
use recommender::{
    Cli, Config, KubernetesLoader, OutputFormat, PrometheusClient, Recommender, RecommenderOutput,
    Result, display_recommendations_table, init_logger,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Install the default crypto provider for rustls
    // I really don't understand why we need this
    // But it was implied in the runtime error message
    // when run without this line :P
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let cli = Cli::parse();

    init_logger(cli.verbose, cli.quiet)?;

    info!("Starting Kubernetes Resource Recommender");
    debug!("AWS Managed Prometheus URL: {}", cli.amp_url);
    debug!("AWS Region: {}", cli.region);

    // Create unified config with all settings
    let config = Config::new(
        String::from(cli.amp_url.clone()),
        cli.region.to_string(),
        cli.context,
        cli.namespace,
        cli.lookback_hours,
        cli.cpu_request_percentile,
        cli.cpu_limit_percentile,
        cli.memory_request_percentile,
        cli.memory_limit_percentile,
        cli.safety_margin,
    );

    // Initialize Kubernetes client
    info!("Connecting to Kubernetes cluster...");
    let k8s_loader = KubernetesLoader::new(config.clone()).await?;

    // Get all deployments with their resource specifications
    info!("Scanning deployments for resource requests and limits...");
    let deployments = k8s_loader.get_deployment_resources().await?;

    info!("Found {} deployments", deployments.len());

    debug!("Connecting to AWS Managed Prometheus...");

    // Initialize Prometheus client
    let prom_client = PrometheusClient::new(cli.amp_url.clone(), cli.region).await?;

    info!("Successfully connected to Prometheus");

    // Generate recommendations
    debug!(
        "Generating recommendations based on {} hours of usage data...",
        config.lookback_hours
    );

    let recommender = Recommender::new(prom_client, config.clone());
    let recommendations = recommender
        .generate_recommendations(deployments.clone())
        .await?;

    info!("Generated {} recommendations", recommendations.len());

    // Build unified output structure
    let output = RecommenderOutput::new(
        config.namespace.clone(),
        config.lookback_hours,
        deployments.len(),
        config.cpu_request_percentile,
        config.cpu_limit_percentile,
        config.memory_request_percentile,
        config.memory_limit_percentile,
        config.safety_margin,
        recommendations,
    );

    // Display output based on format
    if !output.recommendations.is_empty() {
        match cli.output {
            OutputFormat::Table => {
                display_recommendations_table(output)?;
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&output).map_err(|e| {
                    recommender::RecommenderError::Config(recommender::ConfigError::InvalidValue(
                        format!("Failed to serialize JSON: {}", e),
                    ))
                })?;
                println!("{}", json);
            }
        }
    } else {
        info!("No recommendations generated");
    }

    Ok(())
}
