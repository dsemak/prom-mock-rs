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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test QueryParams deserialization.
    #[test]
    fn test_query_params_deserialization() {
        let json = r#"{"query": "up"}"#;
        let params: QueryParams = serde_json::from_str(json).expect("valid JSON");
        assert_eq!(params.query, "up");
    }

    /// Test QueryRangeParams deserialization.
    #[test]
    fn test_query_range_params_deserialization() {
        let json = r#"{"query": "up", "start": "1640995200", "end": "1640998800", "step": "30s"}"#;
        let params: QueryRangeParams = serde_json::from_str(json).expect("valid JSON");
        assert_eq!(params.query, "up");
        assert_eq!(params.start, "1640995200");
        assert_eq!(params.end, "1640998800");
        assert_eq!(params.step, "30s");
    }

    /// Test PromApiResponse serialization with success status.
    #[test]
    fn test_prom_api_response_success() {
        let data = serde_json::json!({"resultType": "vector", "result": []});
        let response = PromApiResponse {
            status: "success",
            data: Some(data.clone()),
            warnings: None,
            error_type: None,
            error: None,
        };

        let json = serde_json::to_string(&response).expect("valid structure");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(parsed["status"], "success");
        assert_eq!(parsed["data"], data);
        assert!(parsed.get("warnings").is_none());
        assert!(parsed.get("errorType").is_none());
        assert!(parsed.get("error").is_none());
    }

    /// Test PromApiResponse serialization with error status.
    #[test]
    fn test_prom_api_response_error() {
        let error_type = "bad_data".to_string();
        let error_msg = "invalid query".to_string();
        let response = PromApiResponse {
            status: "error",
            data: None,
            warnings: None,
            error_type: Some(&error_type),
            error: Some(&error_msg),
        };

        let json = serde_json::to_string(&response).expect("valid structure");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(parsed["status"], "error");
        assert_eq!(parsed["errorType"], "bad_data");
        assert_eq!(parsed["error"], "invalid query");
        assert!(parsed.get("data").is_none());
        assert!(parsed.get("warnings").is_none());
    }

    /// Test PromApiResponse with warnings.
    #[test]
    fn test_prom_api_response_with_warnings() {
        let warnings = vec!["warning1".to_string(), "warning2".to_string()];
        let response = PromApiResponse {
            status: "success",
            data: None,
            warnings: Some(&warnings),
            error_type: None,
            error: None,
        };

        let json = serde_json::to_string(&response).expect("valid structure");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(parsed["status"], "success");
        assert_eq!(parsed["warnings"], serde_json::json!(["warning1", "warning2"]));
    }
}
