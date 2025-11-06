//! Kubernetes Resource Recommender Library
//!
//! This library provides functionality to analyze Kubernetes pod resource usage
//! from AWS Managed Prometheus and generate rightsizing recommendations.

pub mod lib {
    pub mod aws_region;
    pub mod cli;
    pub mod error;
}

// Re-export commonly used types at the root level for convenience
pub use lib::aws_region::AwsRegion;
pub use lib::cli::Cli;
pub use lib::error::{
    AwsError, ConfigError, KubernetesError, PrometheusError, RecommenderError, Result,
};
