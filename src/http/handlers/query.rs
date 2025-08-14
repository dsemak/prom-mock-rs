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
