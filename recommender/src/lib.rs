//! Kubernetes Resource Recommender Library
//!
//! This library provides functionality to analyze Kubernetes pod resource usage
//! from AWS Managed Prometheus and generate rightsizing recommendations.

pub mod lib {
    pub mod aws_region;
    pub mod cli;
    pub mod config;
    pub mod error;
    pub mod kubernetes;
    pub mod logger;
    pub mod output;
    pub mod prometheus;
    pub mod recommender;
    pub mod tui;
}

// Re-export commonly used types at the root level for convenience
pub use lib::aws_region::AwsRegion;
pub use lib::cli::{Cli, OutputFormat};
pub use lib::config::Config;
pub use lib::error::{
    AwsError, ConfigError, KubernetesError, PrometheusError, RecommenderError, Result,
};
pub use lib::kubernetes::{ContainerResources, DeploymentResources, KubernetesLoader};
pub use lib::logger::init_logger;
pub use lib::output::{OutputMetadata, PercentileConfig, RecommenderOutput};
pub use lib::prometheus::{PrometheusClient, PrometheusData, PrometheusResponse, PrometheusResult};
pub use lib::recommender::{Recommender, ResourceRecommendation, UsageStats};
pub use lib::tui::display_recommendations_table;
