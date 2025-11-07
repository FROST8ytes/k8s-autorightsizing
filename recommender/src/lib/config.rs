#[derive(Clone, Debug)]
pub struct Config {
    pub amp_url: String,
    pub region: String,
    pub context: Option<String>,
    pub namespace: Option<String>,
    // Recommender configuration
    pub lookback_hours: u64,
    pub cpu_request_percentile: f64,
    pub cpu_limit_percentile: f64,
    pub memory_request_percentile: f64,
    pub memory_limit_percentile: f64,
    pub safety_margin: f64,
}

impl Config {
    pub fn new(
        amp_url: String,
        region: String,
        context: Option<String>,
        namespace: Option<String>,
        lookback_hours: u64,
        cpu_request_percentile: f64,
        cpu_limit_percentile: f64,
        memory_request_percentile: f64,
        memory_limit_percentile: f64,
        safety_margin: f64,
    ) -> Self {
        Self {
            amp_url,
            region,
            context,
            namespace,
            lookback_hours,
            cpu_request_percentile,
            cpu_limit_percentile,
            memory_request_percentile,
            memory_limit_percentile,
            safety_margin,
        }
    }
}
