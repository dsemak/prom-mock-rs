//! Command-line interface definitions for the Prometheus mock server.

use std::path::PathBuf;

use clap::Parser;
use time::OffsetDateTime;

/// Command-line arguments for the Prometheus mock server.
///
/// This structure defines all command-line options and their default values
/// for configuring the mock server behavior.
#[derive(Debug, Parser)]
#[command(name = "prom-mock")]
#[command(
    author,
    version,
    about = "Simple Prometheus API mock: query and query_range with fixtures"
)]
pub struct Cli {
    /// Address to listen on
    #[arg(long, default_value = "127.0.0.1:19090")]
    pub listen: String,

    /// Path to YAML fixtures file
    #[arg(long)]
    pub fixtures: Option<PathBuf>,

    /// Fixed "now" time (ISO-8601, e.g. 2025-08-03T00:00:00Z)
    #[arg(long, value_parser = parse_time)]
    pub fixed_now: Option<OffsetDateTime>,

    /// Artificial latency for each request (e.g. 100ms, 1s)
    #[arg(long, value_parser = humantime::parse_duration, default_value = "0s")]
    pub latency: std::time::Duration,

    /// Error probability (0.0..1.0). When triggered, returns 503.
    #[arg(long, default_value_t = 0.0)]
    pub error_rate: f32,
}

/// Parse time string into `OffsetDateTime`.
///
/// # Parameters
///
/// - `s` - Time string in RFC3339 format (e.g., "2025-08-03T00:00:00Z")
///
/// # Returns
///
/// Returns parsed `OffsetDateTime` on success, or error message on failure.
///
/// # Errors
///
/// Returns error if the input string is not a valid RFC3339 timestamp.
fn parse_time(s: &str) -> Result<OffsetDateTime, String> {
    time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .map_err(|e| format!("invalid datetime: {e}"))
}
