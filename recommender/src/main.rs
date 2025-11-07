use clap::Parser;
use log::{debug, info};
use recommender::{
    Cli, Config, KubernetesLoader, OutputFormat, PrometheusClient, Recommender, ResourceData,
    Result, display_table, init_logger,
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

    // Display recommendations
    if !recommendations.is_empty() {
        match cli.output {
            OutputFormat::Table => {
                // TODO: Create a better table view for recommendations
                let json = serde_json::to_string_pretty(&recommendations).map_err(|e| {
                    recommender::RecommenderError::Config(recommender::ConfigError::InvalidValue(
                        format!("Failed to serialize JSON: {}", e),
                    ))
                })?;
                println!("{}", json);
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&recommendations).map_err(|e| {
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

    // Convert deployment resources to ResourceData for display
    let mut resource_data: Vec<ResourceData> = Vec::new();

    for deployment in deployments {
        debug!(
            "Processing deployment: {}/{}",
            deployment.namespace, deployment.name
        );

        for container in deployment.containers {
            resource_data.push(ResourceData {
                deployment: deployment.name.clone(),
                container: container.name.clone(),
                namespace: deployment.namespace.clone(),
                cpu_request: container
                    .cpu_request
                    .unwrap_or_else(|| "not set".to_string()),
                cpu_limit: container.cpu_limit.unwrap_or_else(|| "not set".to_string()),
                memory_request: container
                    .memory_request
                    .unwrap_or_else(|| "not set".to_string()),
                memory_limit: container
                    .memory_limit
                    .unwrap_or_else(|| "not set".to_string()),
            });
        }
    }

    // Sort by namespace, deployment, then container
    resource_data.sort_by(|a, b| {
        a.namespace
            .cmp(&b.namespace)
            .then(a.deployment.cmp(&b.deployment))
            .then(a.container.cmp(&b.container))
    });

    info!(
        "Found {} containers across all deployments",
        resource_data.len()
    );

    // Display results based on output format
    if !resource_data.is_empty() {
        match cli.output {
            OutputFormat::Table => {
                display_table(resource_data)?;
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&resource_data).map_err(|e| {
                    recommender::RecommenderError::Config(recommender::ConfigError::InvalidValue(
                        format!("Failed to serialize JSON: {}", e),
                    ))
                })?;
                println!("{}", json);
            }
        }
    } else {
        info!("No containers found in deployments");
    }

    Ok(())
}
