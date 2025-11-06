use k8s_openapi::api::apps::v1::Deployment;
use kube::{Client, Config, config::KubeConfigOptions};
use log::{debug, info};

use crate::{
    Config as RecommenderConfig, ConfigError::InvalidValue, KubernetesError::ApiError,
    KubernetesError::ConnectionFailed, Result,
};

#[derive(Debug, Clone)]
pub struct DeploymentResources {
    pub name: String,
    pub namespace: String,
    pub containers: Vec<ContainerResources>,
}

#[derive(Debug, Clone)]
pub struct ContainerResources {
    pub name: String,
    pub cpu_request: Option<String>,
    pub cpu_limit: Option<String>,
    pub memory_request: Option<String>,
    pub memory_limit: Option<String>,
}

pub struct KubernetesLoader {
    client: Client,
    config: RecommenderConfig,
}

impl KubernetesLoader {
    pub async fn new(config: RecommenderConfig) -> Result<Self> {
        let client = if let Some(ref context) = config.context {
            debug!("Using custom context for Kubeconfig");
            let custom_config = Config::from_kubeconfig(&KubeConfigOptions {
                context: Some(context.clone()),
                ..Default::default()
            })
            .await
            .map_err(|e| InvalidValue(e.to_string()))?;

            debug!("Creating a Kubernetes client using custom Kubeconfig");
            Client::try_from(custom_config).map_err(|e| ConnectionFailed(e.to_string()))?
        } else {
            debug!("Creating a Kubernetes client using default Kubeconfig");
            Client::try_default()
                .await
                .map_err(|e| ConnectionFailed(e.to_string()))?
        };

        info!("Successfully created Kubernetes client");
        Ok(Self { client, config })
    }

    pub async fn get_deployments(&self) -> Result<Vec<String>> {
        let lp = kube::api::ListParams::default();
        let deployments = if let Some(namespace) = self.config.namespace.as_deref() {
            debug!("Listing all deployments in {namespace} namespace");
            let api: kube::Api<k8s_openapi::api::apps::v1::Deployment> =
                kube::Api::namespaced(self.client.clone(), namespace);
            api.list(&lp).await.map_err(|e| ApiError(e.to_string()))?
        } else {
            debug!("Listing all deployments in all namespaces");
            let api: kube::Api<k8s_openapi::api::apps::v1::Deployment> =
                kube::Api::all(self.client.clone());
            api.list(&lp).await.map_err(|e| ApiError(e.to_string()))?
        };

        info!("Retrieved all deployments");
        Ok(deployments
            .items
            .into_iter()
            .filter_map(|d| d.metadata.name)
            .collect())
    }

    pub async fn get_deployment_resources(&self) -> Result<Vec<DeploymentResources>> {
        let lp = kube::api::ListParams::default();
        let deployments = if let Some(namespace) = self.config.namespace.as_deref() {
            debug!("Listing all deployments with resources in {namespace} namespace");
            let api: kube::Api<Deployment> = kube::Api::namespaced(self.client.clone(), namespace);
            api.list(&lp).await.map_err(|e| ApiError(e.to_string()))?
        } else {
            debug!("Listing all deployments with resources in all namespaces");
            let api: kube::Api<Deployment> = kube::Api::all(self.client.clone());
            api.list(&lp).await.map_err(|e| ApiError(e.to_string()))?
        };

        let mut deployment_resources = Vec::new();

        for deployment in deployments.items {
            let name = deployment.metadata.name.unwrap_or_default();
            let namespace = deployment.metadata.namespace.unwrap_or_default();

            if let Some(spec) = deployment.spec {
                if let Some(template) = spec.template.spec {
                    let containers: Vec<ContainerResources> = template
                        .containers
                        .iter()
                        .map(|container| {
                            let resources = container.resources.as_ref();
                            ContainerResources {
                                name: container.name.clone(),
                                cpu_request: resources
                                    .and_then(|r| r.requests.as_ref())
                                    .and_then(|req| req.get("cpu"))
                                    .map(|q| q.0.clone()),
                                cpu_limit: resources
                                    .and_then(|r| r.limits.as_ref())
                                    .and_then(|lim| lim.get("cpu"))
                                    .map(|q| q.0.clone()),
                                memory_request: resources
                                    .and_then(|r| r.requests.as_ref())
                                    .and_then(|req| req.get("memory"))
                                    .map(|q| q.0.clone()),
                                memory_limit: resources
                                    .and_then(|r| r.limits.as_ref())
                                    .and_then(|lim| lim.get("memory"))
                                    .map(|q| q.0.clone()),
                            }
                        })
                        .collect();

                    deployment_resources.push(DeploymentResources {
                        name,
                        namespace,
                        containers,
                    });
                }
            }
        }

        info!(
            "Retrieved {} deployments with resource specs",
            deployment_resources.len()
        );
        Ok(deployment_resources)
    }
}
