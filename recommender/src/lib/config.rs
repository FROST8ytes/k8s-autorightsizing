use url::Url;

use crate::{ConfigError, RecommenderError, Result};

#[derive(Clone, Debug)]
pub struct KubernetesConfig {
    pub amp_url: String,
    pub region: String,
    pub context: Option<String>,
    pub namespace: Option<String>,
}

impl KubernetesConfig {
    pub fn new(
        amp_url: String,
        region: String,
        context: Option<String>,
        namespace: Option<String>,
    ) -> Self {
        Self {
            amp_url,
            region,
            context,
            namespace,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RecommenderConfig {
    pub lookback_hours: f64,
    pub cpu_request_percentile: f64,
    pub cpu_limit_percentile: f64,
    pub memory_request_percentile: f64,
    pub memory_limit_percentile: f64,
    pub safety_margin: f64,
}

impl RecommenderConfig {
    pub fn new(
        lookback_hours: f64,
        cpu_request_percentile: f64,
        cpu_limit_percentile: f64,
        memory_request_percentile: f64,
        memory_limit_percentile: f64,
        safety_margin: f64,
    ) -> Self {
        Self {
            lookback_hours,
            cpu_request_percentile,
            cpu_limit_percentile,
            memory_request_percentile,
            memory_limit_percentile,
            safety_margin,
        }
    }
}

#[derive(Debug, Clone)]
pub enum GitConnectionType {
    Ssh,
    Https,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GitProvider {
    GitHub,
    GitLab,
    Bitbucket,
    Gitea,
    Generic, // For any other Git provider
}

impl GitProvider {
    /// Detect provider from URL
    pub fn from_url(url: &Url) -> Self {
        let url_str = url.as_str();

        if url_str.contains("github.com") {
            GitProvider::GitHub
        } else if url_str.contains("gitlab.com") || url_str.contains("gitlab") {
            GitProvider::GitLab
        } else if url_str.contains("bitbucket.org") {
            GitProvider::Bitbucket
        } else if url_str.contains("gitea") {
            GitProvider::Gitea
        } else {
            GitProvider::Generic
        }
    }

    /// Get the API base URL for the provider
    pub fn api_base_url(&self, git_url: &Url) -> Option<String> {
        match self {
            GitProvider::GitHub => {
                // Extract base domain (supports GitHub Enterprise)
                let host = git_url.host_str()?;
                if host.contains("github.com") {
                    Some("https://api.github.com".to_string())
                } else {
                    // GitHub Enterprise
                    Some(format!("https://{}/api/v3", host))
                }
            }
            GitProvider::GitLab => {
                let host = git_url.host_str()?;
                if host.contains("gitlab.com") {
                    Some("https://gitlab.com/api/v4".to_string())
                } else {
                    // Self-hosted GitLab
                    Some(format!("https://{}/api/v4", host))
                }
            }
            GitProvider::Bitbucket => Some("https://api.bitbucket.org/2.0".to_string()),
            GitProvider::Gitea => {
                let host = git_url.host_str()?;
                Some(format!("https://{}/api/v1", host))
            }
            GitProvider::Generic => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UpdaterConfig {
    pub git_url: Url,
    pub connection_type: GitConnectionType,
    pub auth_token: Option<String>,
    pub auth_username: Option<String>,
    pub provider: GitProvider,
}

impl UpdaterConfig {
    pub fn new(
        git_url: Url,
        auth_token: Option<String>,
        auth_username: Option<String>,
    ) -> Result<Self> {
        let connection_type = match git_url.scheme() {
            "ssh" => Ok(GitConnectionType::Ssh),
            "https" | "http" => Ok(GitConnectionType::Https),
            scheme => Err(RecommenderError::Config(ConfigError::InvalidValue(
                format!("Unsupported git URL scheme: {}", scheme),
            ))),
        }?;

        let provider = GitProvider::from_url(&git_url);

        Ok(Self {
            git_url,
            connection_type,
            auth_token,
            auth_username,
            provider,
        })
    }

    /// Create config with explicit provider override
    pub fn with_provider(
        git_url: Url,
        auth_token: Option<String>,
        auth_username: Option<String>,
        provider: GitProvider,
    ) -> Result<Self> {
        let connection_type = match git_url.scheme() {
            "ssh" => Ok(GitConnectionType::Ssh),
            "https" | "http" => Ok(GitConnectionType::Https),
            scheme => Err(RecommenderError::Config(ConfigError::InvalidValue(
                format!("Unsupported git URL scheme: {}", scheme),
            ))),
        }?;

        Ok(Self {
            git_url,
            connection_type,
            auth_token,
            auth_username,
            provider,
        })
    }
}
