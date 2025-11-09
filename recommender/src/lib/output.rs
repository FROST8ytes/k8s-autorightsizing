use serde::Serialize;

use crate::lib::recommender::ResourceRecommendation;

/// Top-level output structure containing metadata and recommendations
#[derive(Debug, Clone, Serialize)]
pub struct RecommenderOutput {
    pub metadata: OutputMetadata,
    pub recommendations: Vec<ResourceRecommendation>,
}

/// Metadata about the recommendation generation
#[derive(Debug, Clone, Serialize)]
pub struct OutputMetadata {
    pub timestamp: String,
    pub namespace: Option<String>,
    pub lookback_hours: f64,
    pub total_deployments: usize,
    pub total_containers: usize,
    pub percentiles_used: PercentileConfig,
}

/// Configuration for percentiles used in recommendations
#[derive(Debug, Clone, Serialize)]
pub struct PercentileConfig {
    pub cpu_request: f64,
    pub cpu_limit: f64,
    pub memory_request: f64,
    pub memory_limit: f64,
    pub safety_margin: f64,
}

impl RecommenderOutput {
    /// Create a new RecommenderOutput
    pub fn new(
        namespace: Option<String>,
        lookback_hours: f64,
        total_deployments: usize,
        cpu_request_percentile: f64,
        cpu_limit_percentile: f64,
        memory_request_percentile: f64,
        memory_limit_percentile: f64,
        safety_margin: f64,
        recommendations: Vec<ResourceRecommendation>,
    ) -> Self {
        let total_containers = recommendations.len();

        Self {
            metadata: OutputMetadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                namespace,
                lookback_hours,
                total_deployments,
                total_containers,
                percentiles_used: PercentileConfig {
                    cpu_request: cpu_request_percentile,
                    cpu_limit: cpu_limit_percentile,
                    memory_request: memory_request_percentile,
                    memory_limit: memory_limit_percentile,
                    safety_margin,
                },
            },
            recommendations,
        }
    }
}
