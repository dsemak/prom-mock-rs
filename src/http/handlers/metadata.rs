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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::extract::{Path, State};

    use crate::fixtures::FixtureBook;
    use crate::http::state::AppState;
    use crate::storage::{Label, MemoryStorage, Sample, Storage, TimeSeries};

    use super::*;

    fn create_test_state_with_data() -> AppState {
        let storage = Arc::new(MemoryStorage::new());

        // Add test series with different labels
        let mut ts1 = TimeSeries::new(vec![
            Label::new("__name__".to_string(), "test_metric".to_string()),
            Label::new("job".to_string(), "api".to_string()),
            Label::new("instance".to_string(), "localhost:8080".to_string()),
        ]);
        ts1.add_sample(Sample::new(1640995200000, 10.0));

        let mut ts2 = TimeSeries::new(vec![
            Label::new("__name__".to_string(), "another_metric".to_string()),
            Label::new("job".to_string(), "worker".to_string()),
            Label::new("instance".to_string(), "localhost:8081".to_string()),
            Label::new("environment".to_string(), "prod".to_string()),
        ]);
        ts2.add_sample(Sample::new(1640995200000, 20.0));

        storage.add_series(ts1);
        storage.add_series(ts2);

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

    /// Test series endpoint with data.
    #[tokio::test]
    async fn test_series_with_data() {
        let state = create_test_state_with_data();

        let response = series(State(state)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "success");
        assert!(json["data"].is_array());

        let series_data = json["data"].as_array().expect("data is array");
        assert_eq!(series_data.len(), 2);

        // Check that each series has the expected structure
        for series in series_data {
            assert!(series.is_object());
            let obj = series.as_object().expect("series is object");
            assert!(obj.contains_key("__name__"));
            assert!(obj.contains_key("job"));
            assert!(obj.contains_key("instance"));
        }
    }

    /// Test series endpoint with empty storage.
    #[tokio::test]
    async fn test_series_empty() {
        let state = create_test_state_empty();

        let response = series(State(state)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "success");
        assert!(json["data"].is_array());

        let series_data = json["data"].as_array().expect("data is array");
        assert_eq!(series_data.len(), 0);
    }

    /// Test labels endpoint with data.
    #[tokio::test]
    async fn test_labels_with_data() {
        let state = create_test_state_with_data();

        let response = labels(State(state)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "success");
        assert!(json["data"].is_array());

        let labels_data = json["data"].as_array().expect("data is array");
        // Should contain at least __name__, job, instance, environment
        assert!(labels_data.len() >= 4);

        // Convert to strings for easier checking
        let label_names: Vec<String> =
            labels_data.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();

        assert!(label_names.contains(&"__name__".to_string()));
        assert!(label_names.contains(&"job".to_string()));
        assert!(label_names.contains(&"instance".to_string()));
        assert!(label_names.contains(&"environment".to_string()));
    }

    /// Test labels endpoint with empty storage.
    #[tokio::test]
    async fn test_labels_empty() {
        let state = create_test_state_empty();

        let response = labels(State(state)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "success");
        assert!(json["data"].is_array());

        let labels_data = json["data"].as_array().expect("data is array");
        assert_eq!(labels_data.len(), 0);
    }

    /// Test label_values endpoint with existing label.
    #[tokio::test]
    async fn test_label_values_existing_label() {
        let state = create_test_state_with_data();

        let response = label_values(State(state), Path("job".to_string())).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "success");
        assert!(json["data"].is_array());

        let values_data = json["data"].as_array().expect("data is array");
        assert_eq!(values_data.len(), 2); // "api" and "worker"

        let values: Vec<String> =
            values_data.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();

        assert!(values.contains(&"api".to_string()));
        assert!(values.contains(&"worker".to_string()));
    }

    /// Test label_values endpoint with nonexistent label.
    #[tokio::test]
    async fn test_label_values_nonexistent_label() {
        let state = create_test_state_with_data();

        let response = label_values(State(state), Path("nonexistent_label".to_string())).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "success");
        assert!(json["data"].is_array());

        let values_data = json["data"].as_array().expect("data is array");
        assert_eq!(values_data.len(), 0);
    }

    /// Test label_values endpoint with __name__ label.
    #[tokio::test]
    async fn test_label_values_metric_names() {
        let state = create_test_state_with_data();

        let response = label_values(State(state), Path("__name__".to_string())).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "success");
        assert!(json["data"].is_array());

        let values_data = json["data"].as_array().expect("data is array");
        assert_eq!(values_data.len(), 2); // "test_metric" and "another_metric"

        let values: Vec<String> =
            values_data.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();

        assert!(values.contains(&"test_metric".to_string()));
        assert!(values.contains(&"another_metric".to_string()));
    }

    /// Test metadata endpoints with error rate simulation.
    #[tokio::test]
    async fn test_metadata_with_error_simulation() {
        let storage = Arc::new(MemoryStorage::new());
        let state = AppState::builder()
            .with_storage(storage)
            .with_error_rate(1.0) // 100% error rate
            .build()
            .expect("valid configuration");

        // Test series endpoint
        let response = series(State(state.clone())).await;
        let response = response.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);

        // Test labels endpoint
        let response = labels(State(state.clone())).await;
        let response = response.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);

        // Test label_values endpoint
        let response = label_values(State(state), Path("job".to_string())).await;
        let response = response.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    /// Test label_values with empty storage.
    #[tokio::test]
    async fn test_label_values_empty_storage() {
        let state = create_test_state_empty();

        let response = label_values(State(state), Path("job".to_string())).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let (_, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse JSON");

        assert_eq!(json["status"], "success");
        assert!(json["data"].is_array());

        let values_data = json["data"].as_array().expect("data is array");
        assert_eq!(values_data.len(), 0);
    }
}
