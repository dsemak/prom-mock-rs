//! HTTP routing configuration for all API endpoints.

use axum::{
    routing::{get, post},
    Router,
};

use crate::http::handlers::*;
use crate::http::state::AppState;

/// Build the Axum router with all API endpoints.
///
/// # Parameters
///
/// - `state` - Application state containing configuration and dependencies
///
/// # Returns
///
/// Returns configured Axum `Router` with all Prometheus API endpoints.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        // Prometheus Query API (original fixture-based)
        .route("/api/v1/query", get(query))
        .route("/api/v1/query_range", get(query_range))
        // Additional Prometheus API endpoints
        .route("/api/v1/series", get(series))
        .route("/api/v1/labels", get(labels))
        .route("/api/v1/label/{name}/values", get(label_values))
        // Remote Write API
        .route("/api/v1/write", post(remote_write))
        // Query API with in-memory storage fallback
        .route("/api/v1/query_simple", get(query_simple))
        .route("/api/v1/query_range_simple", get(query_range_simple))
        .with_state(state)
}
