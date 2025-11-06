use crate::lib::aws_region::AwsRegion;
use crate::lib::error::{PrometheusError, Result};
use aws_credential_types::Credentials;
use aws_credential_types::provider::ProvideCredentials;
use aws_sigv4::http_request::{SignableBody, SignableRequest, SigningSettings};
use aws_sigv4::sign::v4;
use aws_smithy_runtime_api::client::identity::Identity;
use reqwest::{Client, Method, Request};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use url::Url;

/// Prometheus client with AWS SigV4 authentication
pub struct PrometheusClient {
    client: Client,
    endpoint: Url,
    region: AwsRegion,
    credentials: Credentials,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrometheusResponse {
    pub status: String,
    pub data: PrometheusData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrometheusData {
    #[serde(rename = "resultType")]
    pub result_type: String,
    pub result: Vec<PrometheusResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrometheusResult {
    pub metric: std::collections::HashMap<String, String>,
    pub value: Option<(f64, String)>,
    pub values: Option<Vec<(f64, String)>>,
}

impl PrometheusClient {
    /// Create a new Prometheus client with AWS credentials
    pub async fn new(endpoint: Url, region: AwsRegion) -> Result<Self> {
        // Load AWS credentials from environment
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let credentials = config
            .credentials_provider()
            .ok_or(PrometheusError::AuthenticationFailed)?
            .provide_credentials()
            .await
            .map_err(|_| PrometheusError::AuthenticationFailed)?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| PrometheusError::ConnectionError(e.to_string()))?;

        Ok(Self {
            client,
            endpoint,
            region,
            credentials,
        })
    }

    /// Execute a PromQL query
    pub async fn query(&self, query: &str) -> Result<PrometheusResponse> {
        let mut url = self.endpoint.clone();
        url.set_path(&format!(
            "{}/api/v1/query",
            url.path().trim_end_matches('/')
        ));
        url.query_pairs_mut().append_pair("query", query);

        self.execute_request(Method::GET, url).await
    }

    /// Execute a PromQL range query
    pub async fn query_range(
        &self,
        query: &str,
        start: SystemTime,
        end: SystemTime,
        step: Duration,
    ) -> Result<PrometheusResponse> {
        let mut url = self.endpoint.clone();
        url.set_path(&format!(
            "{}/api/v1/query_range",
            url.path().trim_end_matches('/')
        ));

        let start_secs = start
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let end_secs = end
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        url.query_pairs_mut()
            .append_pair("query", query)
            .append_pair("start", &start_secs.to_string())
            .append_pair("end", &end_secs.to_string())
            .append_pair("step", &format!("{}s", step.as_secs()));

        self.execute_request(Method::GET, url).await
    }

    /// Execute a signed HTTP request
    async fn execute_request(&self, method: Method, url: Url) -> Result<PrometheusResponse> {
        // Create the request
        let mut request = Request::new(method, url.clone());

        // Sign the request with AWS SigV4
        let signable_request = SignableRequest::new(
            request.method().as_str(),
            url.as_str(),
            std::iter::empty(),
            SignableBody::Bytes(&[]),
        )
        .map_err(|e| PrometheusError::ConnectionError(e.to_string()))?;

        let signing_settings = SigningSettings::default();
        let identity: Identity = self.credentials.clone().into();
        let signing_params = v4::SigningParams::builder()
            .identity(&identity)
            .region(self.region.as_str())
            .name("aps")
            .time(SystemTime::now())
            .settings(signing_settings)
            .build()
            .map_err(|e| PrometheusError::ConnectionError(e.to_string()))?
            .into();

        let (signing_instructions, _) =
            aws_sigv4::http_request::sign(signable_request, &signing_params)
                .map_err(|e| PrometheusError::ConnectionError(e.to_string()))?
                .into_parts();

        // Apply signature headers
        for (name, value) in signing_instructions.headers() {
            let header_name: reqwest::header::HeaderName = name.parse().unwrap();
            let header_value: reqwest::header::HeaderValue = value.parse().unwrap();
            request.headers_mut().insert(header_name, header_value);
        }

        // Execute the request
        let response = self
            .client
            .execute(request)
            .await
            .map_err(|e| PrometheusError::ConnectionError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(PrometheusError::QueryError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ))
            .into());
        }

        // Parse response
        let prom_response: PrometheusResponse = response
            .json()
            .await
            .map_err(|e| PrometheusError::QueryError(e.to_string()))?;

        if prom_response.status != "success" {
            return Err(PrometheusError::QueryError(format!(
                "Prometheus returned status: {}",
                prom_response.status
            ))
            .into());
        }

        Ok(prom_response)
    }
}
