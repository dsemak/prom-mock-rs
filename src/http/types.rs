//! API types and parameters for HTTP handlers.

use serde::{Deserialize, Serialize};

/// Query parameters for the `/api/v1/query` endpoint.
#[derive(Debug, Deserialize)]
pub struct QueryParams {
    /// PromQL query string
    pub query: String,
}

/// Query range parameters for the `/api/v1/query_range` endpoint.
#[derive(Debug, Deserialize)]
pub struct QueryRangeParams {
    /// PromQL query string
    pub query: String,
    /// Start time (Unix timestamp or relative)
    pub start: String,
    /// End time (Unix timestamp or relative)
    pub end: String,
    /// Query resolution step
    pub step: String,
}

/// Prometheus API response structure.
#[derive(Debug, Serialize)]
pub struct PromApiResponse<'a> {
    /// Response status ("success" | "error")
    pub status: &'a str,
    /// Response data payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Warning messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<&'a Vec<String>>,
    /// Error type
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "errorType")]
    pub error_type: Option<&'a String>,
    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'a String>,
}
