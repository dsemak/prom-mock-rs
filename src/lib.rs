//! # Prometheus Mock Library
//!
//! A library for creating mock Prometheus HTTP API servers for integration testing.
//!
//! This library provides components for:
//! - **Fixture-based API Mock**: Returns predefined responses from YAML fixtures
//! - **Remote Write Sink**: Accepts remote write data and stores it in memory for querying
//! - **Label Matching**: Extensible label filtering for time series queries
//! - **In-Memory Storage**: Fast storage backend for metrics data
//!
//! # Examples
//!
//! ```no_run
//! use std::sync::Arc;
//! use prom_mock_rs::{MemoryStorage, SimpleQueryEngine, http::build_router};
//!
//! # async fn example() -> std::io::Result<()> {
//! // Create storage and query engine
//! let storage = Arc::new(MemoryStorage::new());
//! let engine = SimpleQueryEngine::new(storage.clone());
//!
//! // Build HTTP router with state
//! let state = prom_mock_rs::http::AppState::builder()
//!     .with_storage(storage)
//!     .build()?;
//! let app = build_router(state);
//! # Ok(())
//! # }
//! ```

pub mod fixtures;
pub mod http;
pub mod matchers;
pub mod query_engine;
pub mod storage;
pub mod timeutil;

// Re-export commonly used types for convenience
pub use fixtures::FixtureBook;
pub use matchers::{EqualMatcher, LabelMatcher, NotEqualMatcher, NotRegexMatcher, RegexMatcher};
pub use query_engine::SimpleQueryEngine;
pub use storage::{Label, MemoryStorage, Sample, Storage, TimeSeries};
