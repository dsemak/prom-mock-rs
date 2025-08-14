//! Health check and utility handlers.

use axum::http::StatusCode;
use tokio::time::sleep;

use crate::http::state::AppState;

/// Health check endpoint.
///
/// # Returns
///
/// Returns "ok" if the server is healthy.
pub async fn healthz() -> &'static str {
    "ok"
}

/// Adds artificial latency and simulates error rate for testing.
///
/// # Parameters
///
/// - `state` - Application state with latency and error rate configuration
///
/// # Returns
///
/// Returns `Ok(())` if no error is simulated, or `Err(StatusCode::SERVICE_UNAVAILABLE)` if an error is triggered.
pub async fn maybe_latency_and_error(state: &AppState) -> Result<(), StatusCode> {
    if !state.mock.latency.is_zero() {
        sleep(state.mock.latency).await;
    }
    if state.mock.error_rate > 0.0 && rand::random::<f32>() < state.mock.error_rate {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    Ok(())
}
