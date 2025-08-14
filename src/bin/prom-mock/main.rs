//! # Prometheus Mock Server CLI
//!
//! Command-line interface for the Prometheus mock server.
//!
//! This binary provides a CLI interface for running the mock server with various
//! configuration options including fixtures, latency simulation, and error injection.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

use prom_mock_rs::fixtures::FixtureBook;
use prom_mock_rs::http::{build_router, AppState};
use prom_mock_rs::storage::MemoryStorage;

mod cli;

use cli::Cli;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize logging
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(env_filter).init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Load fixtures (can work without file - empty set)
    let book = if let Some(path) = &cli.fixtures {
        FixtureBook::load_from_path(path)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
    } else {
        FixtureBook::default()
    };

    // Create in-memory storage for remote write
    let storage = Arc::new(MemoryStorage::new());

    let mut builder = AppState::builder()
        .with_storage(storage)
        .with_fixtures(book)
        .with_latency(cli.latency)
        .with_error_rate(cli.error_rate);

    if let Some(fixed_time) = cli.fixed_now {
        builder = builder.with_fixed_now(fixed_time);
    }

    let state = builder.build()?;

    let app = build_router(state);

    let addr: SocketAddr = cli.listen.parse().map_err(io::Error::other)?;
    tracing::info!("starting prom-mock on http://{addr}");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
