//! Metadata API handlers for series, labels, and label values.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::http::handlers::health::maybe_latency_and_error;
use crate::http::state::AppState;
use crate::http::types::PromApiResponse;

/// Get series matching label selectors.
///
/// # Parameters
///
/// - `state` - Application state containing storage
///
/// # Returns
///
/// Returns series data from storage as JSON response.
pub async fn series(State(state): State<AppState>) -> impl IntoResponse {
    if let Err(code) = maybe_latency_and_error(&state).await {
        return (code, "simulated failure").into_response();
    }

    let series = state.query.storage.query_series(&[]);
    let series_data: Vec<serde_json::Value> = series
        .iter()
        .map(|ts| {
            let mut labels = serde_json::Map::new();
            for label in &ts.labels {
                labels.insert(label.name.clone(), serde_json::Value::String(label.value.clone()));
            }
            serde_json::Value::Object(labels)
        })
        .collect();

    (
        StatusCode::OK,
        Json(PromApiResponse {
            status: "success",
            data: Some(serde_json::Value::Array(series_data)),
            warnings: None,
            error_type: None,
            error: None,
        }),
    )
        .into_response()
}

/// Get all label names.
///
/// # Parameters
///
/// - `state` - Application state containing storage
///
/// # Returns
///
/// Returns array of label names as JSON response.
pub async fn labels(State(state): State<AppState>) -> impl IntoResponse {
    if let Err(code) = maybe_latency_and_error(&state).await {
        return (code, "simulated failure").into_response();
    }

    let names = state.query.storage.label_names();
    (
        StatusCode::OK,
        Json(PromApiResponse {
            status: "success",
            data: Some(serde_json::Value::Array(
                names.into_iter().map(serde_json::Value::String).collect(),
            )),
            warnings: None,
            error_type: None,
            error: None,
        }),
    )
        .into_response()
}

/// Get values for a specific label.
///
/// # Parameters
///
/// - `state` - Application state containing storage
/// - `label_name` - Name of the label to get values for
///
/// # Returns
///
/// Returns array of label values as JSON response.
pub async fn label_values(
    State(state): State<AppState>,
    Path(label_name): Path<String>,
) -> impl IntoResponse {
    if let Err(code) = maybe_latency_and_error(&state).await {
        return (code, "simulated failure").into_response();
    }

    let values = state.query.storage.label_values(&label_name);
    (
        StatusCode::OK,
        Json(PromApiResponse {
            status: "success",
            data: Some(serde_json::Value::Array(
                values.into_iter().map(serde_json::Value::String).collect(),
            )),
            warnings: None,
            error_type: None,
            error: None,
        }),
    )
        .into_response()
}
