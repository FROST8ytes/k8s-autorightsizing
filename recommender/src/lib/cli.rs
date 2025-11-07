use clap::Parser;
use url::Url;

use crate::AwsRegion;

/// Kubernetes Resource Recommender
///
/// Analyzes pod resource usage from AWS Managed Prometheus and generates
/// rightsizing recommendations.
#[derive(Parser, Debug)]
#[command(name = "recommender", author, version, about, styles=get_styles())]
pub struct Cli {
    /// Amazon Managed Prometheus workspace endpoint
    #[arg(long, value_name = "URL")]
    pub amp_url: Url,

    /// AWS Region
    #[arg(short, long)]
    pub region: AwsRegion,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppress log output to stdout/stderr (logs still written to file)
    #[arg(short, long)]
    pub quiet: bool,

    /// Provide context name
    ///
    /// Use if you have multiple clusters in your kubeconfig
    #[arg(long)]
    pub context: Option<String>,

    /// Namespace to scan workloads for rightsizing
    #[arg(long)]
    pub namespace: Option<String>,

    /// Output format: table (default) or json
    #[arg(long, value_name = "FORMAT", default_value = "table")]
    pub output: OutputFormat,

    /// Lookback period in hours for recommendations (default: 168 = 7 days)
    #[arg(long, default_value = "168")]
    pub lookback_hours: u64,

    /// CPU percentile for request recommendations (default: 95)
    #[arg(long, default_value = "95.0")]
    pub cpu_request_percentile: f64,

    /// CPU percentile for limit recommendations (default: 99)
    #[arg(long, default_value = "99.0")]
    pub cpu_limit_percentile: f64,

    /// Memory percentile for request recommendations (default: 95)
    #[arg(long, default_value = "95.0")]
    pub memory_request_percentile: f64,

    /// Memory percentile for limit recommendations (default: 99)
    #[arg(long, default_value = "99.0")]
    pub memory_limit_percentile: f64,

    /// Safety margin multiplier for recommendations (default: 1.2 = 20% buffer)
    #[arg(long, default_value = "1.2")]
    pub safety_margin: f64,
}

/// Output format for the recommender results
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    /// Display results in an interactive table (TUI)
    Table,
    /// Output results as JSON
    Json,
}

/// Set color and variants for help description
///
/// Thanks to [Praveen Perera](https://stackoverflow.com/a/76916424)
fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .header(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .literal(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .invalid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .error(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .valid(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .placeholder(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
}
