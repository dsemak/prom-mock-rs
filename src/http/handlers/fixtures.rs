//! Fixture-based handlers for mocking Prometheus API responses.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::fixtures::QueryParams as FQueryParams;
use crate::http::handlers::health::maybe_latency_and_error;
use crate::http::state::AppState;
use crate::http::types::{PromApiResponse, QueryParams, QueryRangeParams};

/// Handle instant query requests using fixtures.
///
/// # Parameters
///
/// - `state` - Application state containing fixture data
/// - `params` - Query parameters from request
///
/// # Returns
///
/// Returns fixture response if matching fixture is found, otherwise 404.
pub async fn query(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    if let Err(code) = maybe_latency_and_error(&state).await {
        return (code, "simulated failure").into_response();
    }

    let qp = FQueryParams { query: params.query.clone(), start: None, end: None, step: None };

    if let Some(resp) = state.mock.fixtures.find_match("/api/v1/query", &qp, state.mock.fixed_now) {
        let status = state.mock.fixtures.effective_status(resp);
        return (
            StatusCode::OK,
            Json(PromApiResponse {
                status,
                data: Some(resp.data.clone()),
                warnings: resp.warnings.as_ref(),
                error_type: resp.error_type.as_ref(),
                error: resp.error.as_ref(),
            }),
        )
            .into_response();
    }

    // No match found - return 404 in Prometheus style
    (
        StatusCode::NOT_FOUND,
        Json(PromApiResponse {
            status: "error",
            data: None,
            warnings: None,
            error_type: Some(&"not_found".to_string()),
            error: Some(&"no fixture matched".to_string()),
        }),
    )
        .into_response()
}

/// Handle query range requests using fixtures.
///
/// # Parameters
///
/// - `state` - Application state containing fixture data
/// - `params` - Query range parameters from request
///
/// # Returns
///
/// Returns fixture response if matching fixture is found, otherwise 404.
pub async fn query_range(
    State(state): State<AppState>,
    Query(params): Query<QueryRangeParams>,
) -> impl IntoResponse {
    if let Err(code) = maybe_latency_and_error(&state).await {
        return (code, "simulated failure").into_response();
    }

    // Normalize input: if relative values came in and we have fixed_now - resolve them.
    let start = stringify_resolved(&params.start, state.mock.fixed_now);
    let end = stringify_resolved(&params.end, state.mock.fixed_now);

    let qp = FQueryParams {
        query: params.query.clone(),
        start: Some(start),
        end: Some(end),
        step: Some(params.step.clone()),
    };

    if let Some(resp) =
        state.mock.fixtures.find_match("/api/v1/query_range", &qp, state.mock.fixed_now)
    {
        let status = state.mock.fixtures.effective_status(resp);
        return (
            StatusCode::OK,
            Json(PromApiResponse {
                status,
                data: Some(resp.data.clone()),
                warnings: resp.warnings.as_ref(),
                error_type: resp.error_type.as_ref(),
                error: resp.error.as_ref(),
            }),
        )
            .into_response();
    }

    (
        StatusCode::NOT_FOUND,
        Json(PromApiResponse {
            status: "error",
            data: None,
            warnings: None,
            error_type: Some(&"not_found".to_string()),
            error: Some(&"no fixture matched".to_string()),
        }),
    )
        .into_response()
}

/// Convert relative time parameters to string format.
fn stringify_resolved(input: &str, now: Option<time::OffsetDateTime>) -> String {
    match crate::timeutil::resolve_relative(input, now) {
        crate::timeutil::ResolvedParam::Absolute(s)
        | crate::timeutil::ResolvedParam::Relative(s)
        | crate::timeutil::ResolvedParam::Raw(s) => s,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::extract::{Query, State};

    use crate::fixtures::{FixtureBook, Matcher, Respond, Route};
    use crate::http::state::AppState;
    use crate::storage::MemoryStorage;

    use super::*;

    fn create_test_state_with_fixtures() -> AppState {
        let mut fixtures = FixtureBook::default();

        // Add test fixture for instant query
        let query_fixture = Route {
            matcher: Matcher {
                path: "/api/v1/query".to_string(),
                query: Some("up".to_string()),
                start: None,
                end: None,
                step: None,
            },
            respond: Respond {
                status: None,
                data: serde_json::json!({
                    "resultType": "vector",
                    "result": [
                        {
                            "metric": {"__name__": "up", "job": "test"},
                            "value": [1640995200, "1"]
                        }
                    ]
                }),
                warnings: None,
                error_type: None,
                error: None,
            },
        };

        // Add test fixture for range query
        let range_fixture = Route {
            matcher: Matcher {
                path: "/api/v1/query_range".to_string(),
                query: Some("up".to_string()),
                start: Some("1640995200".to_string()),
                end: Some("1640998800".to_string()),
                step: Some("30s".to_string()),
            },
            respond: Respond {
                status: None,
                data: serde_json::json!({
                    "resultType": "matrix",
                    "result": [
                        {
                            "metric": {"__name__": "up", "job": "test"},
                            "values": [
                                [1640995200, "1"],
                                [1640995230, "1"]
                            ]
                        }
                    ]
                }),
                warnings: None,
                error_type: None,
                error: None,
            },
        };

        fixtures.routes = vec![query_fixture, range_fixture];

        let storage = Arc::new(MemoryStorage::new());
        AppState::builder()
            .with_storage(storage)
            .with_fixtures(fixtures)
            .build()
            .expect("valid configuration")
    }

    fn create_test_state_empty_fixtures() -> AppState {
        let storage = Arc::new(MemoryStorage::new());
        AppState::builder()
            .with_storage(storage)
            .with_fixtures(FixtureBook::default())
            .build()
            .expect("valid configuration")
    }

    /// Test query handler with matching fixture.
    #[tokio::test]
    async fn test_query_with_matching_fixture() {
        let state = create_test_state_with_fixtures();
        let params = QueryParams { query: "up".to_string() };

        let response = query(State(state), Query(params)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        // Test that we can read the response body
        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "success");
        assert_eq!(json["data"]["resultType"], "vector");
    }

    /// Test query handler without matching fixture.
    #[tokio::test]
    async fn test_query_without_matching_fixture() {
        let state = create_test_state_empty_fixtures();
        let params = QueryParams { query: "nonexistent_metric".to_string() };

        let response = query(State(state), Query(params)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "error");
        assert_eq!(json["errorType"], "not_found");
        assert_eq!(json["error"], "no fixture matched");
    }

    /// Test query_range handler with matching fixture.
    #[tokio::test]
    async fn test_query_range_with_matching_fixture() {
        let state = create_test_state_with_fixtures();
        let params = QueryRangeParams {
            query: "up".to_string(),
            start: "1640995200".to_string(),
            end: "1640998800".to_string(),
            step: "30s".to_string(),
        };

        let response = query_range(State(state), Query(params)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "success");
        assert_eq!(json["data"]["resultType"], "matrix");
    }

    /// Test query_range handler without matching fixture.
    #[tokio::test]
    async fn test_query_range_without_matching_fixture() {
        let state = create_test_state_empty_fixtures();
        let params = QueryRangeParams {
            query: "nonexistent_metric".to_string(),
            start: "1640995200".to_string(),
            end: "1640998800".to_string(),
            step: "30s".to_string(),
        };

        let response = query_range(State(state), Query(params)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "error");
        assert_eq!(json["errorType"], "not_found");
        assert_eq!(json["error"], "no fixture matched");
    }

    /// Test stringify_resolved function.
    #[test]
    fn test_stringify_resolved() {
        let now = time::OffsetDateTime::from_unix_timestamp(1640995200).expect("valid timestamp");

        // Test absolute timestamp
        let result = stringify_resolved("1640995200", Some(now));
        assert_eq!(result, "1640995200");

        // Test relative time expression
        let result = stringify_resolved("now-1h", Some(now));
        // Should resolve to a specific timestamp
        assert!(!result.is_empty());
        assert_ne!(result, "now-1h");
    }

    /// Test query with error rate simulation.
    #[tokio::test]
    async fn test_query_with_error_simulation() {
        let storage = Arc::new(MemoryStorage::new());
        let state = AppState::builder()
            .with_storage(storage)
            .with_error_rate(1.0) // 100% error rate
            .build()
            .expect("valid configuration");

        let params = QueryParams { query: "up".to_string() };

        let response = query(State(state), Query(params)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    /// Test query_range with error rate simulation.
    #[tokio::test]
    async fn test_query_range_with_error_simulation() {
        let storage = Arc::new(MemoryStorage::new());
        let state = AppState::builder()
            .with_storage(storage)
            .with_error_rate(1.0) // 100% error rate
            .build()
            .expect("valid configuration");

        let params = QueryRangeParams {
            query: "up".to_string(),
            start: "1640995200".to_string(),
            end: "1640998800".to_string(),
            step: "30s".to_string(),
        };

        let response = query_range(State(state), Query(params)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    /// Test fixture response with warnings.
    #[tokio::test]
    async fn test_query_with_warnings_fixture() {
        let mut fixtures = FixtureBook::default();

        let warning_fixture = Route {
            matcher: Matcher {
                path: "/api/v1/query".to_string(),
                query: Some("warning_metric".to_string()),
                start: None,
                end: None,
                step: None,
            },
            respond: Respond {
                status: None,
                data: serde_json::json!({"resultType": "vector", "result": []}),
                warnings: Some(vec!["This is a warning".to_string()]),
                error_type: None,
                error: None,
            },
        };

        fixtures.routes = vec![warning_fixture];

        let storage = Arc::new(MemoryStorage::new());
        let state = AppState::builder()
            .with_storage(storage)
            .with_fixtures(fixtures)
            .build()
            .expect("valid configuration");

        let params = QueryParams { query: "warning_metric".to_string() };

        let response = query(State(state), Query(params)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "success");
        assert!(json["warnings"].is_array());
        assert_eq!(json["warnings"][0], "This is a warning");
    }
}
