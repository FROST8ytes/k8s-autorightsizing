use clap::Parser;
use log::{debug, info, warn};
use recommender::{
    Cli, KubernetesConfig, KubernetesLoader, ManifestUpdater, OutputFormat, PrometheusClient,
    Recommender, RecommenderConfig, RecommenderOutput, ResourceRecommendation, Result,
    UpdaterConfig, display_recommendations_table, init_logger,
};
use std::io::{self, Write};

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
    let k8s_config = KubernetesConfig::new(
        String::from(cli.amp_url.clone()),
        cli.region.to_string(),
        cli.context,
        cli.namespace,
    );
    let recommender_config = RecommenderConfig::new(
        cli.lookback_hours,
        cli.cpu_request_percentile,
        cli.cpu_limit_percentile,
        cli.memory_request_percentile,
        cli.memory_limit_percentile,
        cli.safety_margin,
    );

    // Initialize Kubernetes client
    info!("Connecting to Kubernetes cluster...");
    let k8s_loader = KubernetesLoader::new(k8s_config.clone()).await?;

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
        recommender_config.lookback_hours
    );

    let recommender = Recommender::new(prom_client, recommender_config.clone());
    let recommendations = recommender
        .generate_recommendations(deployments.clone())
        .await?;

    info!("Generated {} recommendations", recommendations.len());

    // Build unified output structure
    let output = RecommenderOutput::new(
        k8s_config.namespace.clone(),
        recommender_config.lookback_hours,
        deployments.len(),
        recommender_config.cpu_request_percentile,
        recommender_config.cpu_limit_percentile,
        recommender_config.memory_request_percentile,
        recommender_config.memory_limit_percentile,
        recommender_config.safety_margin,
        recommendations,
    );

    // Display output based on format
    if !output.recommendations.is_empty() {
        // Always output JSON for logging purposes
        let json = serde_json::to_string_pretty(&output).map_err(|e| {
            recommender::RecommenderError::Config(recommender::ConfigError::InvalidValue(format!(
                "Failed to serialize JSON: {}",
                e
            )))
        })?;

        info!("Recommendations JSON: {}", json);

        // Phase 1: Automatic apply mode (only for non-table output)
        if cli.apply && cli.manifest_url.is_some() && cli.output != OutputFormat::Table {
            info!("Automatic apply mode enabled");
            apply_recommendations_automatic(
                cli.manifest_url.unwrap(),
                cli.git_branch,
                cli.git_username,
                cli.git_token,
                &output.recommendations,
            )
            .await?;
            return Ok(());
        }

        // Display based on output format
        match cli.output {
            OutputFormat::Table => {
                display_recommendations_table(
                    output,
                    cli.manifest_url,
                    cli.git_branch,
                    cli.git_username,
                    cli.git_token,
                )?;
            }
            OutputFormat::Json => {
                info!("{}", json);

                // Phase 3: Interactive CLI mode for JSON output
                if cli.apply {
                    apply_recommendations_interactive_cli(
                        cli.manifest_url,
                        cli.git_branch,
                        cli.git_token,
                        &output.recommendations,
                    )
                    .await?;
                }
            }
        }
    } else {
        info!("No recommendations generated");
    }

    Ok(())
}

/// Apply recommendations automatically (non-interactive mode)
async fn apply_recommendations_automatic(
    manifest_url: url::Url,
    git_branch: String,
    git_username: Option<String>,
    git_token: Option<String>,
    recommendations: &[ResourceRecommendation],
) -> Result<()> {
    info!("Creating updater configuration...");

    let updater_config = UpdaterConfig::new(manifest_url.clone(), git_token, git_username)?;
    let mut updater = ManifestUpdater::new(updater_config)?;

    info!("Applying recommendations and creating PR...");
    let (branch_name, _commit_sha, pr_url) = updater
        .apply_and_create_pr(&git_branch, recommendations)
        .await?;

    info!("Successfully created branch: {}", branch_name);
    if let Some(url) = pr_url {
        info!("Pull Request created: {}", url);
    } else {
        warn!(
            "Changes committed to branch '{}' but PR creation was not available",
            branch_name
        );
    }

    Ok(())
}

/// Apply recommendations with interactive CLI prompts (for JSON mode)
async fn apply_recommendations_interactive_cli(
    manifest_url: Option<url::Url>,
    git_branch: String,
    git_token: Option<String>,
    recommendations: &[ResourceRecommendation],
) -> Result<()> {
    // Prompt 1: Confirm apply
    print!(
        "\nApply changes to all {} containers? (y/n): ",
        recommendations.len()
    );
    io::stdout().flush().unwrap();

    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm).map_err(|e| {
        recommender::RecommenderError::Other(format!("Failed to read input: {}", e))
    })?;

    if !confirm.trim().eq_ignore_ascii_case("y") {
        info!("Apply cancelled by user");
        return Ok(());
    }

    // Prompt 2: Get Git URL if not provided
    let url = if let Some(url) = manifest_url {
        url
    } else {
        print!("Enter Git repository URL: ");
        io::stdout().flush().unwrap();

        let mut url_input = String::new();
        io::stdin().read_line(&mut url_input).map_err(|e| {
            recommender::RecommenderError::Other(format!("Failed to read input: {}", e))
        })?;

        url::Url::parse(url_input.trim()).map_err(|e| {
            recommender::RecommenderError::Config(recommender::ConfigError::InvalidValue(format!(
                "Invalid URL: {}",
                e
            )))
        })?
    };

    // Prompt 3: Get token if not provided
    let token = if let Some(token) = git_token {
        Some(token)
    } else {
        print!("Enter Git token (optional, press Enter for public repo): ");
        io::stdout().flush().unwrap();

        let mut token_input = String::new();
        io::stdin().read_line(&mut token_input).map_err(|e| {
            recommender::RecommenderError::Other(format!("Failed to read input: {}", e))
        })?;

        let trimmed = token_input.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    };

    // Prompt 4: Confirm branch
    print!("Enter branch name (default: {}): ", git_branch);
    io::stdout().flush().unwrap();

    let mut branch_input = String::new();
    io::stdin().read_line(&mut branch_input).map_err(|e| {
        recommender::RecommenderError::Other(format!("Failed to read input: {}", e))
    })?;

    let branch = if branch_input.trim().is_empty() {
        git_branch
    } else {
        branch_input.trim().to_string()
    };

    // Execute apply
    info!("Creating updater configuration...");
    let updater_config = UpdaterConfig::new(url.clone(), token, None)?;
    let mut updater = ManifestUpdater::new(updater_config)?;

    let (branch_name, _commit_sha, pr_url) = updater
        .apply_and_create_pr(&branch, recommendations)
        .await?;

    // Output result as JSON
    let result = serde_json::json!({
        "status": "success",
        "branch": branch_name,
        "pr_url": pr_url,
    });

    info!("\n{}", serde_json::to_string_pretty(&result).unwrap());

    Ok(())
}
