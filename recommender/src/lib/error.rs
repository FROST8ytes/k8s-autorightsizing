use thiserror::Error;

/// Main error type for the recommender application
#[derive(Error, Debug)]
pub enum RecommenderError {
    /// AWS-related errors
    #[error("AWS error: {0}")]
    Aws(#[from] AwsError),

    /// Prometheus query errors
    #[error("Prometheus error: {0}")]
    Prometheus(#[from] PrometheusError),

    /// Kubernetes API errors
    #[error("Kubernetes error: {0}")]
    Kubernetes(#[from] KubernetesError),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Parsing errors
    #[error("Parse error: {0}")]
    Parse(String),

    /// Network/HTTP errors
    #[error("Network error: {0}")]
    Network(String),

    /// Invalid input/arguments
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Insufficient data for analysis
    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    /// Generic error with message
    #[error("{0}")]
    Other(String),
}

/// AWS-specific errors
#[derive(Error, Debug)]
pub enum AwsError {
    /// Authentication failure
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Authorization/permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// AWS service error
    #[error("AWS service error: {0}")]
    ServiceError(String),

    /// Invalid AWS region
    #[error("Invalid region: {0}")]
    InvalidRegion(String),

    /// AWS resource not found
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// AWS rate limiting
    #[error("Rate limited: {0}")]
    RateLimited(String),
}

/// Prometheus-specific errors
#[derive(Error, Debug)]
pub enum PrometheusError {
    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Connection error (generic)
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Authentication failed
    #[error("Authentication failed")]
    AuthenticationFailed,

    /// Query syntax error
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Query execution error
    #[error("Query failed: {0}")]
    QueryFailed(String),

    /// Query error (generic)
    #[error("Query error: {0}")]
    QueryError(String),

    /// No data returned
    #[error("No data: {0}")]
    NoData(String),

    /// Invalid response format
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Timeout
    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Kubernetes-specific errors
#[derive(Error, Debug)]
pub enum KubernetesError {
    /// API server connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// Invalid resource specification
    #[error("Invalid resource: {0}")]
    InvalidResource(String),

    /// API error
    #[error("API error: {0}")]
    ApiError(String),
}

/// Configuration-specific errors
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Missing required configuration
    #[error("Missing required: {0}")]
    MissingRequired(String),

    /// Invalid configuration value
    #[error("Invalid value: {0}")]
    InvalidValue(String),

    /// Configuration file error
    #[error("File error: {0}")]
    FileError(String),
}

/// Helper type alias for Results
pub type Result<T> = std::result::Result<T, RecommenderError>;
