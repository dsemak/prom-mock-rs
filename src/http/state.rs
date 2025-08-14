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
#[derive(Clone, Debug)]
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use crate::fixtures::FixtureBook;
    use crate::storage::MemoryStorage;

    use super::*;

    fn create_test_storage() -> Arc<dyn FullStorage> {
        Arc::new(MemoryStorage::new())
    }

    /// Test QueryConfig creation and initialization.
    #[test]
    fn test_query_config_new() {
        let storage = create_test_storage();
        let fixed_now = Some(time::OffsetDateTime::now_utc());

        let config = QueryConfig::new(storage.clone(), fixed_now);

        assert!(Arc::ptr_eq(&config.storage, &storage));
        assert_eq!(config.fixed_now, fixed_now);
    }

    /// Test MockConfig creation with all parameters.
    #[test]
    fn test_mock_config_new() {
        let fixtures = FixtureBook::default();
        let latency = Duration::from_millis(100);
        let error_rate = 0.5;
        let fixed_now = Some(time::OffsetDateTime::now_utc());

        let config = MockConfig::new(fixtures.clone(), latency, error_rate, fixed_now);

        assert_eq!(config.latency, latency);
        assert_eq!(config.error_rate, error_rate);
        assert_eq!(config.fixed_now, fixed_now);
    }

    /// Test AppState creation with valid parameters.
    #[test]
    fn test_app_state_new() {
        let fixtures = FixtureBook::default();
        let storage = create_test_storage();
        let fixed_now = Some(time::OffsetDateTime::now_utc());
        let latency = Duration::from_millis(50);
        let error_rate = 0.1;

        let state = AppState::new(fixtures, fixed_now, latency, error_rate, storage);

        assert_eq!(state.mock.latency, latency);
        assert_eq!(state.mock.error_rate, error_rate);
        assert_eq!(state.mock.fixed_now, fixed_now);
        assert_eq!(state.query.fixed_now, fixed_now);
    }

    /// Test AppStateBuilder default creation.
    #[test]
    fn test_app_state_builder_new() {
        let builder = AppStateBuilder::new();
        assert!(builder.storage.is_none());
        assert!(builder.fixtures.is_none());
        assert!(builder.fixed_now.is_none());
        assert!(builder.latency.is_none());
        assert!(builder.error_rate.is_none());
    }

    /// Test AppStateBuilder with_storage.
    #[test]
    fn test_app_state_builder_with_storage() {
        let storage = create_test_storage();
        let builder = AppStateBuilder::new().with_storage(storage.clone());

        assert!(builder.storage.is_some());
        assert!(Arc::ptr_eq(builder.storage.as_ref().unwrap(), &storage));
    }

    /// Test AppStateBuilder with_fixtures.
    #[test]
    fn test_app_state_builder_with_fixtures() {
        let fixtures = FixtureBook::default();
        let builder = AppStateBuilder::new().with_fixtures(fixtures.clone());

        assert!(builder.fixtures.is_some());
    }

    /// Test AppStateBuilder with_fixed_now.
    #[test]
    fn test_app_state_builder_with_fixed_now() {
        let now = time::OffsetDateTime::now_utc();
        let builder = AppStateBuilder::new().with_fixed_now(now);

        assert_eq!(builder.fixed_now, Some(now));
    }

    /// Test AppStateBuilder with_latency.
    #[test]
    fn test_app_state_builder_with_latency() {
        let latency = Duration::from_millis(200);
        let builder = AppStateBuilder::new().with_latency(latency);

        assert_eq!(builder.latency, Some(latency));
    }

    /// Test AppStateBuilder with_error_rate.
    #[test]
    fn test_app_state_builder_with_error_rate() {
        let error_rate = 0.3;
        let builder = AppStateBuilder::new().with_error_rate(error_rate);

        assert_eq!(builder.error_rate, Some(error_rate));
    }

    /// Test AppStateBuilder successful build.
    #[test]
    fn test_app_state_builder_build_success() {
        let storage = create_test_storage();
        let fixtures = FixtureBook::default();
        let now = time::OffsetDateTime::now_utc();
        let latency = Duration::from_millis(100);
        let error_rate = 0.2;

        let result = AppStateBuilder::new()
            .with_storage(storage)
            .with_fixtures(fixtures)
            .with_fixed_now(now)
            .with_latency(latency)
            .with_error_rate(error_rate)
            .build();

        assert!(result.is_ok());
        let state = result.unwrap();
        assert_eq!(state.mock.latency, latency);
        assert_eq!(state.mock.error_rate, error_rate);
        assert_eq!(state.mock.fixed_now, Some(now));
    }

    /// Test AppStateBuilder build without storage - should fail.
    #[test]
    fn test_app_state_builder_build_missing_storage() {
        let result = AppStateBuilder::new().build();

        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert!(error.to_string().contains("Storage is required"));
        }
    }

    /// Test AppStateBuilder build with invalid error rate - should fail.
    #[test]
    fn test_app_state_builder_build_invalid_error_rate() {
        let storage = create_test_storage();

        // Test error rate > 1.0
        let result =
            AppStateBuilder::new().with_storage(storage.clone()).with_error_rate(1.5).build();

        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert!(error.to_string().contains("Error rate must be between 0.0 and 1.0"));
        }

        // Test error rate < 0.0
        let result = AppStateBuilder::new().with_storage(storage).with_error_rate(-0.1).build();

        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert!(error.to_string().contains("Error rate must be between 0.0 and 1.0"));
        }
    }

    /// Test AppStateBuilder build with defaults.
    #[test]
    fn test_app_state_builder_build_with_defaults() {
        let storage = create_test_storage();

        let result = AppStateBuilder::new().with_storage(storage).build();

        assert!(result.is_ok());
        let state = result.unwrap();
        assert_eq!(state.mock.latency, Duration::ZERO);
        assert_eq!(state.mock.error_rate, 0.0);
        assert_eq!(state.mock.fixed_now, None);
    }

    /// Test AppState builder method.
    #[test]
    fn test_app_state_builder_method() {
        let builder = AppState::builder();
        assert!(builder.storage.is_none());
    }

    /// Test method chaining with builder.
    #[test]
    fn test_app_state_builder_chaining() {
        let storage = create_test_storage();
        let now = time::OffsetDateTime::now_utc();

        let result = AppState::builder()
            .with_storage(storage)
            .with_latency(Duration::from_millis(50))
            .with_error_rate(0.1)
            .with_fixed_now(now)
            .with_fixtures(FixtureBook::default())
            .build();

        assert!(result.is_ok());
        let state = result.unwrap();
        assert_eq!(state.mock.latency, Duration::from_millis(50));
        assert_eq!(state.mock.error_rate, 0.1);
        assert_eq!(state.mock.fixed_now, Some(now));
    }
}
