use crate::Result;
use crate::lib::config::RecommenderConfig;
use crate::lib::kubernetes::{ContainerResources, DeploymentResources};
use crate::lib::prometheus::PrometheusClient;
use log::{debug, info};
use serde::Serialize;
use std::time::{Duration, SystemTime};

/// Recommendation for a container's resource sizing
#[derive(Debug, Clone, Serialize)]
pub struct ResourceRecommendation {
    pub deployment: String,
    pub container: String,
    pub namespace: String,
    pub current_cpu_request: String,
    pub current_cpu_limit: String,
    pub current_memory_request: String,
    pub current_memory_limit: String,
    pub recommended_cpu_request: String,
    pub recommended_cpu_limit: String,
    pub recommended_memory_request: String,
    pub recommended_memory_limit: String,
    pub cpu_usage_stats: UsageStats,
    pub memory_usage_stats: UsageStats,
    pub recommendation_reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageStats {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

pub struct Recommender {
    prometheus: PrometheusClient,
    config: RecommenderConfig,
}

impl Recommender {
    pub fn new(prometheus: PrometheusClient, config: RecommenderConfig) -> Self {
        Self { prometheus, config }
    }

    /// Generate recommendations for all deployments
    pub async fn generate_recommendations(
        &self,
        deployments: Vec<DeploymentResources>,
    ) -> Result<Vec<ResourceRecommendation>> {
        let mut recommendations = Vec::new();

        for deployment in deployments {
            info!(
                "Analyzing deployment {}/{} with {} containers",
                deployment.namespace,
                deployment.name,
                deployment.containers.len()
            );

            for container in &deployment.containers {
                match self
                    .generate_container_recommendation(&deployment, &container)
                    .await
                {
                    Ok(rec) => recommendations.push(rec),
                    Err(e) => {
                        debug!(
                            "Failed to generate recommendation for {}/{}/{}: {}",
                            deployment.namespace, deployment.name, container.name, e
                        );
                    }
                }
            }
        }

        Ok(recommendations)
    }

    /// Generate recommendation for a single container
    async fn generate_container_recommendation(
        &self,
        deployment: &DeploymentResources,
        container: &ContainerResources,
    ) -> Result<ResourceRecommendation> {
        debug!(
            "Generating recommendation for container: {}/{}/{}",
            deployment.namespace, deployment.name, container.name
        );

        // Get time range for queries
        let end_time = SystemTime::now();
        let start_time = end_time - Duration::from_secs_f64(self.config.lookback_hours * 3600.0);

        // Query CPU usage
        let cpu_query = format!(
            r#"rate(container_cpu_usage_seconds_total{{namespace="{}",pod=~"{}.*",container="{}"}}[5m])"#,
            deployment.namespace, deployment.name, container.name
        );
        let cpu_usage = self.query_metrics(&cpu_query, start_time, end_time).await?;
        let cpu_stats = self.calculate_stats(&cpu_usage);

        // Query memory usage (in bytes)
        let memory_query = format!(
            r#"container_memory_working_set_bytes{{namespace="{}",pod=~"{}.*",container="{}"}}"#,
            deployment.namespace, deployment.name, container.name
        );
        let memory_usage = self
            .query_metrics(&memory_query, start_time, end_time)
            .await?;
        let memory_stats = self.calculate_stats(&memory_usage);

        // Generate recommendations
        let recommended_cpu_request = self.recommend_cpu_request(&cpu_stats);
        let recommended_cpu_limit = self.recommend_cpu_limit(&cpu_stats);
        let recommended_memory_request = self.recommend_memory_request(&memory_stats);
        let recommended_memory_limit = self.recommend_memory_limit(&memory_stats);

        let recommendation_reason = self.generate_reason(
            &container,
            &cpu_stats,
            &memory_stats,
            &recommended_cpu_request,
            &recommended_memory_request,
        );

        Ok(ResourceRecommendation {
            deployment: deployment.name.clone(),
            container: container.name.clone(),
            namespace: deployment.namespace.clone(),
            current_cpu_request: container
                .cpu_request
                .clone()
                .unwrap_or_else(|| "not set".to_string()),
            current_cpu_limit: container
                .cpu_limit
                .clone()
                .unwrap_or_else(|| "not set".to_string()),
            current_memory_request: container
                .memory_request
                .clone()
                .unwrap_or_else(|| "not set".to_string()),
            current_memory_limit: container
                .memory_limit
                .clone()
                .unwrap_or_else(|| "not set".to_string()),
            recommended_cpu_request,
            recommended_cpu_limit,
            recommended_memory_request,
            recommended_memory_limit,
            cpu_usage_stats: cpu_stats,
            memory_usage_stats: memory_stats,
            recommendation_reason,
        })
    }

    /// Query metrics from Prometheus and extract values
    async fn query_metrics(
        &self,
        query: &str,
        start_time: SystemTime,
        end_time: SystemTime,
    ) -> Result<Vec<f64>> {
        let step = Duration::from_secs(300); // 5 minute intervals
        let response = self
            .prometheus
            .query_range(query, start_time, end_time, step)
            .await?;

        let mut values = Vec::new();
        for result in response.data.result {
            if let Some(vals) = result.values {
                for (_, value_str) in vals {
                    if let Ok(value) = value_str.parse::<f64>() {
                        if value.is_finite() && value >= 0.0 {
                            values.push(value);
                        }
                    }
                }
            }
        }

        debug!(
            "Collected {} data points for query: {}",
            values.len(),
            query
        );
        Ok(values)
    }

    /// Calculate statistics from a set of values
    fn calculate_stats(&self, values: &[f64]) -> UsageStats {
        if values.is_empty() {
            return UsageStats {
                min: 0.0,
                max: 0.0,
                avg: 0.0,
                p50: 0.0,
                p95: 0.0,
                p99: 0.0,
            };
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let avg = sorted.iter().sum::<f64>() / sorted.len() as f64;
        let p50 = self.percentile(&sorted, 50.0);
        let p95 = self.percentile(&sorted, 95.0);
        let p99 = self.percentile(&sorted, 99.0);

        UsageStats {
            min,
            max,
            avg,
            p50,
            p95,
            p99,
        }
    }

    /// Calculate percentile value
    fn percentile(&self, sorted_values: &[f64], percentile: f64) -> f64 {
        if sorted_values.is_empty() {
            return 0.0;
        }
        let index = (percentile / 100.0 * (sorted_values.len() - 1) as f64).ceil() as usize;
        sorted_values[index.min(sorted_values.len() - 1)]
    }

    /// Recommend CPU request based on usage statistics
    fn recommend_cpu_request(&self, stats: &UsageStats) -> String {
        let base_value =
            self.percentile(&[stats.p50, stats.p95], self.config.cpu_request_percentile);
        let recommended = base_value * self.config.safety_margin;
        self.format_cpu_value(recommended)
    }

    /// Recommend CPU limit based on usage statistics
    fn recommend_cpu_limit(&self, stats: &UsageStats) -> String {
        let base_value = self.percentile(&[stats.p95, stats.p99], self.config.cpu_limit_percentile);
        let recommended = base_value * self.config.safety_margin;
        self.format_cpu_value(recommended)
    }

    /// Recommend memory request based on usage statistics
    fn recommend_memory_request(&self, stats: &UsageStats) -> String {
        let base_value = self.percentile(
            &[stats.p50, stats.p95],
            self.config.memory_request_percentile,
        );
        let recommended = base_value * self.config.safety_margin;
        self.format_memory_value(recommended)
    }

    /// Recommend memory limit based on usage statistics
    fn recommend_memory_limit(&self, stats: &UsageStats) -> String {
        let base_value =
            self.percentile(&[stats.p95, stats.p99], self.config.memory_limit_percentile);
        let recommended = base_value * self.config.safety_margin;
        self.format_memory_value(recommended)
    }

    /// Format CPU value in millicores (m) or cores
    fn format_cpu_value(&self, cores: f64) -> String {
        if cores < 0.001 {
            "1m".to_string()
        } else if cores < 1.0 {
            format!("{}m", (cores * 1000.0).ceil() as u64)
        } else {
            format!("{:.2}", cores)
        }
    }

    /// Format memory value in appropriate units (Mi, Gi)
    fn format_memory_value(&self, bytes: f64) -> String {
        const MIB: f64 = 1024.0 * 1024.0;
        const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

        if bytes < MIB {
            "1Mi".to_string()
        } else if bytes < GIB {
            format!("{}Mi", (bytes / MIB).ceil() as u64)
        } else {
            format!("{:.2}Gi", bytes / GIB)
        }
    }

    /// Generate human-readable reason for the recommendation
    fn generate_reason(
        &self,
        container: &ContainerResources,
        cpu_stats: &UsageStats,
        memory_stats: &UsageStats,
        recommended_cpu: &str,
        recommended_memory: &str,
    ) -> String {
        let current_cpu = container.cpu_request.as_deref().unwrap_or("not set");
        let current_memory = container.memory_request.as_deref().unwrap_or("not set");

        let mut reasons = Vec::new();

        // CPU analysis
        if current_cpu == "not set" {
            reasons.push(format!(
                "No CPU request set, recommend {} based on p95 usage",
                recommended_cpu
            ));
        } else if cpu_stats.p95 > 0.0 {
            reasons.push(format!(
                "CPU p95 usage: {:.3} cores, avg: {:.3} cores",
                cpu_stats.p95, cpu_stats.avg
            ));
        }

        // Memory analysis
        if current_memory == "not set" {
            reasons.push(format!(
                "No memory request set, recommend {} based on p95 usage",
                recommended_memory
            ));
        } else if memory_stats.p95 > 0.0 {
            let mem_mib = memory_stats.p95 / (1024.0 * 1024.0);
            reasons.push(format!(
                "Memory p95 usage: {:.0}Mi, avg: {:.0}Mi",
                mem_mib,
                memory_stats.avg / (1024.0 * 1024.0)
            ));
        }

        if reasons.is_empty() {
            "Based on observed usage patterns".to_string()
        } else {
            reasons.join("; ")
        }
    }
}
