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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use crate::fixtures::FixtureBook;
    use crate::http::state::{MockConfig, QueryConfig};
    use crate::query_engine::SimpleQueryEngine;
    use crate::storage::MemoryStorage;

    use super::*;

    /// Test health check endpoint.
    #[tokio::test]
    async fn test_healthz() {
        let result = healthz().await;
        assert_eq!(result, "ok");
    }

    /// Test latency simulation without error.
    #[tokio::test]
    async fn test_latency_no_error() {
        let storage = Arc::new(MemoryStorage::new());
        let state = AppState {
            query: QueryConfig {
                storage: storage.clone(),
                query_engine: SimpleQueryEngine::new(storage),
                fixed_now: None,
            },
            mock: MockConfig {
                latency: Duration::from_millis(10),
                error_rate: 0.0,
                fixtures: std::sync::Arc::new(FixtureBook::default()),
                fixed_now: None,
            },
        };

        let start = std::time::Instant::now();
        let result = maybe_latency_and_error(&state).await;
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert!(elapsed >= Duration::from_millis(10));
    }

    /// Test no latency, no error.
    #[tokio::test]
    async fn test_no_latency_no_error() {
        let state = AppState {
            query: QueryConfig {
                storage: std::sync::Arc::new(crate::storage::MemoryStorage::new()),
                query_engine: SimpleQueryEngine::new(std::sync::Arc::new(
                    crate::storage::MemoryStorage::new(),
                )),
                fixed_now: None,
            },
            mock: MockConfig {
                latency: Duration::ZERO,
                error_rate: 0.0,
                fixtures: std::sync::Arc::new(FixtureBook::default()),
                fixed_now: None,
            },
        };

        let result = maybe_latency_and_error(&state).await;
        assert!(result.is_ok());
    }

    /// Test guaranteed error (error_rate = 1.0).
    #[tokio::test]
    async fn test_guaranteed_error() {
        let state = AppState {
            query: QueryConfig {
                storage: std::sync::Arc::new(crate::storage::MemoryStorage::new()),
                query_engine: SimpleQueryEngine::new(std::sync::Arc::new(
                    crate::storage::MemoryStorage::new(),
                )),
                fixed_now: None,
            },
            mock: MockConfig {
                latency: Duration::ZERO,
                error_rate: 1.0,
                fixtures: std::sync::Arc::new(FixtureBook::default()),
                fixed_now: None,
            },
        };

        let result = maybe_latency_and_error(&state).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
