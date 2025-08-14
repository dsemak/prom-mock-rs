//! Storage-based query handlers for live data queries.

use std::io;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::http::handlers::health::maybe_latency_and_error;
use crate::http::state::AppState;
use crate::http::types::{QueryParams, QueryRangeParams};
use crate::query_engine::QueryResult;
use crate::storage::{Label, Sample};

/// Convert seconds to milliseconds (Prometheus uses millisecond timestamps).
const SECONDS_TO_MILLISECONDS: i64 = 1000;

/// Convert milliseconds to seconds (Prometheus API returns seconds in JSON).
const MILLISECONDS_TO_SECONDS: i64 = 1000;

/// Default query lookback period in milliseconds (5 minutes).
/// When no specific time range is provided, we look back this far from current time.
const DEFAULT_LOOKBACK_MS: i64 = 5 * 60 * 1000; // 5 minutes * 60 seconds * 1000 ms

/// Simple query using in-memory storage.
///
/// # Parameters
///
/// - `state` - Application state containing storage and query engine
/// - `params` - Query parameters from request
///
/// # Returns
///
/// Returns query results from storage as instant vector response.
pub async fn query_simple(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    if let Err(code) = maybe_latency_and_error(&state).await {
        return (code, "simulated failure").into_response();
    }

    let now = state.query.fixed_now.unwrap_or_else(time::OffsetDateTime::now_utc);
    let timestamp = now.unix_timestamp() * SECONDS_TO_MILLISECONDS;

    let query_result =
        state.query.query_engine.query(&params.query, timestamp - DEFAULT_LOOKBACK_MS, timestamp);

    match query_result {
        Ok(result) => build_vector_response(result, timestamp),
        Err(e) => build_error_response(e),
    }
    .into_response()
}

/// Simple query range using in-memory storage.
///
/// # Parameters
///
/// - `state` - Application state containing storage and query engine
/// - `params` - Query range parameters from request
///
/// # Returns
///
/// Returns query results from storage as matrix response.
pub async fn query_range_simple(
    State(state): State<AppState>,
    Query(params): Query<QueryRangeParams>,
) -> impl IntoResponse {
    if let Err(code) = maybe_latency_and_error(&state).await {
        return (code, "simulated failure").into_response();
    }

    let start_ts = parse_time_param(&params.start, state.query.fixed_now);
    let end_ts = parse_time_param(&params.end, state.query.fixed_now);

    let query_result = state.query.query_engine.query(&params.query, start_ts, end_ts);

    match query_result {
        Ok(result) => build_matrix_response(result),
        Err(e) => build_error_response(e),
    }
    .into_response()
}

/// Build a successful vector response for instant queries.
fn build_vector_response(
    result: QueryResult,
    timestamp: i64,
) -> (StatusCode, Json<serde_json::Value>) {
    let series_data = result
        .series
        .iter()
        .map(|ts| {
            let labels = build_labels_map(&ts.labels);
            let value = build_instant_value(&ts.samples, timestamp);

            serde_json::json!({
                "metric": labels,
                "value": value
            })
        })
        .collect::<Vec<_>>();

    let response = serde_json::json!({
        "status": "success",
        "data": {
            "resultType": "vector",
            "result": series_data
        }
    });

    (StatusCode::OK, Json(response))
}

/// Build a successful matrix response for range queries.
fn build_matrix_response(result: QueryResult) -> (StatusCode, Json<serde_json::Value>) {
    let series_data = result
        .series
        .iter()
        .map(|ts| {
            let labels = build_labels_map(&ts.labels);
            let values = build_range_values(&ts.samples);

            serde_json::json!({
                "metric": labels,
                "values": values
            })
        })
        .collect::<Vec<_>>();

    let response = serde_json::json!({
        "status": "success",
        "data": {
            "resultType": "matrix",
            "result": series_data
        }
    });

    (StatusCode::OK, Json(response))
}

/// Build an error response for failed queries.
fn build_error_response(error: io::Error) -> (StatusCode, Json<serde_json::Value>) {
    tracing::warn!("query error: {}", error);

    let response = serde_json::json!({
        "status": "error",
        "errorType": "bad_data",
        "error": error.to_string()
    });

    (StatusCode::BAD_REQUEST, Json(response))
}

/// Convert labels to JSON map.
fn build_labels_map(labels: &[Label]) -> serde_json::Map<String, serde_json::Value> {
    labels
        .iter()
        .map(|label| (label.name.clone(), serde_json::Value::String(label.value.clone())))
        .collect()
}

/// Build instant value for vector queries (latest sample or default).
fn build_instant_value(samples: &[Sample], fallback_timestamp: i64) -> serde_json::Value {
    samples.last().map_or_else(
        || build_sample_array(fallback_timestamp / MILLISECONDS_TO_SECONDS, 0.0),
        |sample| build_sample_array(sample.timestamp / MILLISECONDS_TO_SECONDS, sample.value),
    )
}

/// Build values array for range queries.
fn build_range_values(samples: &[Sample]) -> Vec<serde_json::Value> {
    samples
        .iter()
        .map(|sample| build_sample_array(sample.timestamp / MILLISECONDS_TO_SECONDS, sample.value))
        .collect()
}

/// Build a [timestamp, value] array for Prometheus format.
fn build_sample_array(timestamp_seconds: i64, value: f64) -> serde_json::Value {
    serde_json::Value::Array(vec![
        serde_json::Value::Number(timestamp_seconds.into()),
        serde_json::Value::String(value.to_string()),
    ])
}

/// Parse time parameter to milliseconds timestamp.
fn parse_time_param(param: &str, fixed_now: Option<time::OffsetDateTime>) -> i64 {
    match crate::timeutil::resolve_relative(param, fixed_now) {
        crate::timeutil::ResolvedParam::Absolute(s)
        | crate::timeutil::ResolvedParam::Relative(s) => {
            s.parse::<i64>().unwrap_or(0) * SECONDS_TO_MILLISECONDS
        }
        crate::timeutil::ResolvedParam::Raw(s) => {
            s.parse::<i64>().unwrap_or(0) * SECONDS_TO_MILLISECONDS
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::extract::{Query, State};

    use crate::fixtures::FixtureBook;
    use crate::http::state::AppState;
    use crate::storage::{Label, MemoryStorage, Sample, Storage, TimeSeries};

    use super::*;

    fn create_test_state_with_data() -> AppState {
        let storage = Arc::new(MemoryStorage::new());

        // Add some test data
        let mut ts = TimeSeries::new(vec![
            Label::new("__name__".to_string(), "test_metric".to_string()),
            Label::new("job".to_string(), "test".to_string()),
        ]);

        let now = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        ts.add_sample(Sample::new(now - 60000, 10.0)); // 1 minute ago
        ts.add_sample(Sample::new(now - 30000, 15.0)); // 30 seconds ago
        ts.add_sample(Sample::new(now, 20.0)); // now

        storage.add_series(ts);

        AppState::builder()
            .with_storage(storage)
            .with_fixtures(FixtureBook::default())
            .build()
            .expect("valid configuration")
    }

    fn create_test_state_empty() -> AppState {
        let storage = Arc::new(MemoryStorage::new());
        AppState::builder()
            .with_storage(storage)
            .with_fixtures(FixtureBook::default())
            .build()
            .expect("valid configuration")
    }

    /// Test query_simple with valid data.
    #[tokio::test]
    async fn test_query_simple_with_data() {
        let state = create_test_state_with_data();
        let params = QueryParams { query: "test_metric".to_string() };

        let response = query_simple(State(state), Query(params)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    /// Test query_simple with empty storage.
    #[tokio::test]
    async fn test_query_simple_empty() {
        let state = create_test_state_empty();
        let params = QueryParams { query: "nonexistent_metric".to_string() };

        let response = query_simple(State(state), Query(params)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    /// Test query_range_simple with valid data.
    #[tokio::test]
    async fn test_query_range_simple_with_data() {
        let state = create_test_state_with_data();
        let params = QueryRangeParams {
            query: "test_metric".to_string(),
            start: "1640995200".to_string(), // 2022-01-01 00:00:00 UTC
            end: "1640998800".to_string(),   // 2022-01-01 01:00:00 UTC
            step: "30s".to_string(),
        };

        let response = query_range_simple(State(state), Query(params)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    /// Test query_range_simple with empty storage.
    #[tokio::test]
    async fn test_query_range_simple_empty() {
        let state = create_test_state_empty();
        let params = QueryRangeParams {
            query: "nonexistent_metric".to_string(),
            start: "1640995200".to_string(),
            end: "1640998800".to_string(),
            step: "30s".to_string(),
        };

        let response = query_range_simple(State(state), Query(params)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    /// Test build_vector_response function.
    #[test]
    fn test_build_vector_response() {
        let series = vec![crate::query_engine::QueryResultSeries {
            labels: vec![
                Label::new("__name__".to_string(), "test_metric".to_string()),
                Label::new("job".to_string(), "test".to_string()),
            ],
            samples: vec![Sample::new(1640995200000, 42.0)],
        }];

        let result = crate::query_engine::QueryResult { series };
        let (status, json) = build_vector_response(result, 1640995200000);

        assert_eq!(status, axum::http::StatusCode::OK);

        let value = json.0;
        assert_eq!(value["status"], "success");
        assert_eq!(value["data"]["resultType"], "vector");
        assert!(value["data"]["result"].is_array());
    }

    /// Test build_matrix_response function.
    #[test]
    fn test_build_matrix_response() {
        let series = vec![crate::query_engine::QueryResultSeries {
            labels: vec![Label::new("__name__".to_string(), "test_metric".to_string())],
            samples: vec![Sample::new(1640995200000, 10.0), Sample::new(1640995230000, 15.0)],
        }];

        let result = crate::query_engine::QueryResult { series };
        let (status, json) = build_matrix_response(result);

        assert_eq!(status, axum::http::StatusCode::OK);

        let value = json.0;
        assert_eq!(value["status"], "success");
        assert_eq!(value["data"]["resultType"], "matrix");
        assert!(value["data"]["result"].is_array());
    }

    /// Test build_error_response function.
    #[test]
    fn test_build_error_response() {
        let error = std::io::Error::new(std::io::ErrorKind::InvalidInput, "test error");
        let (status, json) = build_error_response(error);

        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);

        let value = json.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["errorType"], "bad_data");
        assert_eq!(value["error"], "test error");
    }

    /// Test build_labels_map function.
    #[test]
    fn test_build_labels_map() {
        let labels = vec![
            Label::new("__name__".to_string(), "test_metric".to_string()),
            Label::new("job".to_string(), "test".to_string()),
        ];

        let map = build_labels_map(&labels);

        assert_eq!(map.len(), 2);
        assert_eq!(map["__name__"], serde_json::Value::String("test_metric".to_string()));
        assert_eq!(map["job"], serde_json::Value::String("test".to_string()));
    }

    /// Test build_instant_value function.
    #[test]
    fn test_build_instant_value() {
        let samples = vec![Sample::new(1640995200000, 10.0), Sample::new(1640995230000, 15.0)];

        let value = build_instant_value(&samples, 1640995260000);

        // Should use the last sample
        let array = value.as_array().expect("should be array");
        assert_eq!(array.len(), 2);
        assert_eq!(array[0], serde_json::Value::Number(1640995230.into()));
        assert_eq!(array[1], serde_json::Value::String("15".to_string()));
    }

    /// Test build_instant_value with empty samples.
    #[test]
    fn test_build_instant_value_empty() {
        let samples = vec![];
        let fallback_timestamp = 1640995260000;

        let value = build_instant_value(&samples, fallback_timestamp);

        let array = value.as_array().expect("should be array");
        assert_eq!(array.len(), 2);
        assert_eq!(array[0], serde_json::Value::Number(1640995260.into()));
        assert_eq!(array[1], serde_json::Value::String("0".to_string()));
    }

    /// Test build_range_values function.
    #[test]
    fn test_build_range_values() {
        let samples = vec![Sample::new(1640995200000, 10.0), Sample::new(1640995230000, 15.0)];

        let values = build_range_values(&samples);

        assert_eq!(values.len(), 2);

        let first = values[0].as_array().expect("should be array");
        assert_eq!(first[0], serde_json::Value::Number(1640995200.into()));
        assert_eq!(first[1], serde_json::Value::String("10".to_string()));

        let second = values[1].as_array().expect("should be array");
        assert_eq!(second[0], serde_json::Value::Number(1640995230.into()));
        assert_eq!(second[1], serde_json::Value::String("15".to_string()));
    }

    /// Test build_sample_array function.
    #[test]
    fn test_build_sample_array() {
        let value = build_sample_array(1640995200, 42.5);

        let array = value.as_array().expect("should be array");
        assert_eq!(array.len(), 2);
        assert_eq!(array[0], serde_json::Value::Number(1640995200.into()));
        assert_eq!(array[1], serde_json::Value::String("42.5".to_string()));
    }

    /// Test parse_time_param function.
    #[test]
    fn test_parse_time_param() {
        // Test absolute timestamp
        let result = parse_time_param("1640995200", None);
        assert_eq!(result, 1640995200000); // converted to milliseconds

        // Test invalid input (should default to 0)
        let result = parse_time_param("invalid", None);
        assert_eq!(result, 0);
    }

    /// Test query with error rate simulation.
    #[tokio::test]
    async fn test_query_simple_with_error_simulation() {
        let storage = Arc::new(MemoryStorage::new());
        let state = AppState::builder()
            .with_storage(storage)
            .with_error_rate(1.0) // 100% error rate
            .build()
            .expect("valid configuration");

        let params = QueryParams { query: "test_metric".to_string() };

        let response = query_simple(State(state), Query(params)).await;
        let response = response.into_response();

        // Should return SERVICE_UNAVAILABLE due to error simulation
        assert_eq!(response.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }
}
