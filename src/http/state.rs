//! Application state and configuration for the HTTP server.

use std::io;
use std::sync::Arc;

use crate::fixtures::FixtureBook;
use crate::query_engine::SimpleQueryEngine;
use crate::storage::FullStorage;

/// Query-related configuration and dependencies.
///
/// Contains only the dependencies needed for query operations,
/// following the Interface Segregation Principle.
#[derive(Clone)]
pub struct QueryConfig {
    /// Time series storage for remote write data
    pub storage: Arc<dyn FullStorage>,
    /// Query engine for executing metric queries
    pub query_engine: SimpleQueryEngine,
    /// Fixed timestamp for deterministic responses (testing only)
    pub fixed_now: Option<time::OffsetDateTime>,
}

/// Mock behavior configuration for simulation features.
///
/// Contains settings for simulating latency, errors, and fixture responses,
/// separated from core query functionality.
#[derive(Clone)]
pub struct MockConfig {
    /// Fixture data for predefined responses
    pub fixtures: Arc<FixtureBook>,
    /// Artificial delay added to all responses
    pub latency: std::time::Duration,
    /// Probability (0.0-1.0) of returning 503 errors
    pub error_rate: f32,
    /// Fixed timestamp for deterministic responses (testing only)
    pub fixed_now: Option<time::OffsetDateTime>,
}

/// Application state shared across all HTTP handlers.
///
/// Contains specialized configuration objects following the Interface
/// Segregation Principle - handlers only depend on what they need.
#[derive(Clone)]
pub struct AppState {
    /// Query-related configuration
    pub query: QueryConfig,
    /// Mock behavior configuration
    pub mock: MockConfig,
}

impl QueryConfig {
    /// Create new query configuration.
    ///
    /// # Parameters
    ///
    /// - `storage` - Storage implementation for remote write data
    /// - `fixed_now` - Optional fixed timestamp for deterministic testing
    ///
    /// # Returns
    /// Returns configured `QueryConfig` instance with initialized query engine.
    pub fn new(storage: Arc<dyn FullStorage>, fixed_now: Option<time::OffsetDateTime>) -> Self {
        let query_engine = SimpleQueryEngine::new(storage.clone());
        Self { storage, query_engine, fixed_now }
    }
}

impl MockConfig {
    /// Create new mock configuration.
    ///
    /// # Parameters
    ///
    /// - `fixtures` - Fixture definitions for predefined responses
    /// - `latency` - Artificial delay to add to all responses  
    /// - `error_rate` - Probability (0.0-1.0) of returning 503 errors
    /// - `fixed_now` - Optional fixed timestamp for deterministic testing
    ///
    /// # Returns
    /// Returns configured `MockConfig` instance.
    pub fn new(
        fixtures: FixtureBook,
        latency: std::time::Duration,
        error_rate: f32,
        fixed_now: Option<time::OffsetDateTime>,
    ) -> Self {
        Self { fixtures: Arc::new(fixtures), latency, error_rate, fixed_now }
    }
}

impl AppState {
    /// Create new application state with the given configuration.
    ///
    /// # Parameters
    ///
    /// - `fixtures` - Fixture definitions for predefined responses
    /// - `fixed_now` - Optional fixed timestamp for deterministic testing
    /// - `latency` - Artificial delay to add to all responses  
    /// - `error_rate` - Probability (0.0-1.0) of returning 503 errors
    /// - `storage` - Storage implementation for remote write data
    ///
    /// # Returns
    ///
    /// Returns configured `AppState` instance with specialized configurations.
    pub fn new(
        fixtures: FixtureBook,
        fixed_now: Option<time::OffsetDateTime>,
        latency: std::time::Duration,
        error_rate: f32,
        storage: Arc<dyn FullStorage>,
    ) -> Self {
        let query = QueryConfig::new(storage, fixed_now);
        let mock = MockConfig::new(fixtures, latency, error_rate, fixed_now);
        Self { query, mock }
    }

    /// Get a builder for configuring application state step by step.
    ///
    /// # Returns
    ///
    /// Returns an `AppStateBuilder` for fluent configuration.
    pub fn builder() -> AppStateBuilder {
        AppStateBuilder::new()
    }
}

/// Builder for constructing AppState with fluent interface.
///
/// This builder follows the Builder pattern to provide a clean, fluent interface
/// for constructing complex configuration objects with optional parameters.
#[derive(Default)]
pub struct AppStateBuilder {
    storage: Option<Arc<dyn FullStorage>>,
    fixtures: Option<FixtureBook>,
    fixed_now: Option<time::OffsetDateTime>,
    latency: Option<std::time::Duration>,
    error_rate: Option<f32>,
}

impl AppStateBuilder {
    /// Create a new builder with default values.
    ///
    /// # Returns
    ///
    /// Returns a new `AppStateBuilder` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the storage implementation.
    ///
    /// # Parameters
    ///
    /// - `storage` - Storage implementation to use
    ///
    /// # Returns
    ///
    /// Returns the builder for method chaining.
    pub fn with_storage(mut self, storage: Arc<dyn FullStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Set the fixture book for predefined responses.
    ///
    /// # Parameters
    ///
    /// - `fixtures` - Fixture definitions
    ///
    /// # Returns
    /// Returns the builder for method chaining.
    pub fn with_fixtures(mut self, fixtures: FixtureBook) -> Self {
        self.fixtures = Some(fixtures);
        self
    }

    /// Set a fixed timestamp for deterministic testing.
    ///
    /// # Parameters
    ///
    /// - `fixed_now` - Fixed timestamp to use
    ///
    /// # Returns
    ///
    /// Returns the builder for method chaining.
    pub fn with_fixed_now(mut self, fixed_now: time::OffsetDateTime) -> Self {
        self.fixed_now = Some(fixed_now);
        self
    }

    /// Set artificial latency for response simulation.
    ///
    /// # Parameters
    ///
    /// - `latency` - Delay to add to responses
    ///
    /// # Returns
    ///
    /// Returns the builder for method chaining.
    pub fn with_latency(mut self, latency: std::time::Duration) -> Self {
        self.latency = Some(latency);
        self
    }

    /// Set error rate for response simulation.
    ///
    /// # Parameters
    ///
    /// - `error_rate` - Probability (0.0-1.0) of returning errors
    ///
    /// # Returns   
    ///
    /// Returns the builder for method chaining.
    pub fn with_error_rate(mut self, error_rate: f32) -> Self {
        self.error_rate = Some(error_rate);
        self
    }

    /// Build the final AppState with validation.
    ///
    /// # Returns
    ///
    /// Returns `Ok(AppState)` if valid, or `Err(AppStateError)` with specific error type.
    ///
    /// # Errors
    ///
    /// Returns error if storage is not provided or if error_rate is invalid.
    pub fn build(self) -> io::Result<AppState> {
        // Validate required dependencies
        let storage = self.storage.ok_or(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Storage is required for AppState",
        ))?;

        // Validate error rate range
        if let Some(rate) = self.error_rate {
            if !(0.0..=1.0).contains(&rate) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Error rate must be between 0.0 and 1.0, got: {rate}"),
                ));
            }
        }

        // Use defaults for optional values
        let fixtures = self.fixtures.unwrap_or_default();
        let latency = self.latency.unwrap_or_default();
        let error_rate = self.error_rate.unwrap_or(0.0);

        Ok(AppState::new(fixtures, self.fixed_now, latency, error_rate, storage))
    }
}
