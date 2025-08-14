//! HTTP handlers for different API endpoints.

pub mod fixtures;
pub mod health;
pub mod metadata;
pub mod query;
pub mod remote_write;

// Re-export handlers for easier access
pub use fixtures::{query, query_range};
pub use health::healthz;
pub use metadata::{label_values, labels, series};
pub use query::{query_range_simple, query_simple};
pub use remote_write::remote_write;
