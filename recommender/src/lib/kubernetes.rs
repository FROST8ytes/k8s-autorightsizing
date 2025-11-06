use kube::{Client, Config, config::KubeConfigOptions};
use log::{debug, info};

use crate::{
    Config as RecommenderConfig, ConfigError::InvalidValue, KubernetesError::ApiError,
    KubernetesError::ConnectionFailed, Result,
};

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
}
