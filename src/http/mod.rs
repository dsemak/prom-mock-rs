//! HTTP server with Prometheus-compatible API endpoints and configurable mock behavior.

pub mod handlers;
pub mod routes;
pub mod state;
pub mod types;

pub use routes::build_router;
pub use state::AppState;
