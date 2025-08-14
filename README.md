# prom-mock-rs

A mock Prometheus HTTP API server for integration testing, available both as a **library** and **CLI application**.

## Overview

This project provides a lightweight mock implementation of the Prometheus HTTP API, designed for testing applications that integrate with Prometheus. It supports configurable response fixtures, artificial latency, error injection, and in-memory metrics storage.

## Features

- **Remote Write API**: Compatible with Prometheus remote write protocol
- **Query API**: Basic query endpoint support for stored metrics
- **Fixture System**: YAML-based predefined responses
- **Mock Behavior**: Configurable latency and error rates
- **In-Memory Storage**: Temporary metrics storage for testing scenarios
- **Library API**: Use as a Rust library in your tests
- **CLI Application**: Standalone server for integration testing

## Installation

### Prerequisites

This library requires Rust 1.76.0 or later.

### As a CLI tool

```bash
cargo install prom-mock-rs
```

### As a library dependency

```toml
[dependencies]
prom-mock-rs = "0.1.0"
```

## Usage

### CLI Application

#### Basic Usage

```bash
prom-mock --listen 127.0.0.1:9090
```

#### With Fixtures

```bash
prom-mock --fixtures fixtures.yaml --listen 127.0.0.1:9090
```

#### Configuration Options

- `--listen`: Address to listen on (default: 127.0.0.1:19090)
- `--fixtures`: Path to YAML fixture file
- `--latency`: Artificial response delay (e.g., 100ms, 1s)
- `--error-rate`: Probability of 503 errors (0.0-1.0)
- `--fixed-now`: Fixed "now" time for testing (ISO-8601 format)

### Library Usage

```rust
use std::sync::Arc;
use prom_mock_rs::{MemoryStorage, http::build_router};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Create storage and app state
    let storage = Arc::new(MemoryStorage::new());
    let state = prom_mock_rs::http::AppState::builder()
        .with_storage(storage)
        .build()?;
    
    // Build the router
    let app = build_router(state);
    
    // Serve with axum
    let listener = tokio::net::TcpListener::bind("127.0.0.1:19090").await?;
    axum::serve(listener, app).await
}
```

## API Endpoints

- `POST /api/v1/write` - Remote write endpoint
- `GET /api/v1/query` - Query endpoint
- `GET /health` - Health check

## Fixture Format

```yaml
version: 1
defaults:
  status: 200
  latency: "100ms"
routes:
  - path: "/api/v1/query"
    method: "GET"
    response:
      status: 200
      body: |
        {"status":"success","data":{"resultType":"vector","result":[]}}
```

## Development

```bash
# Run tests
cargo test

# Run the CLI with development settings
cargo run --bin prom-mock -- --listen 127.0.0.1:9090 --fixtures tests/fixtures.yaml

# Build the library
cargo build

# Check library documentation
cargo doc --open
```

## Library API

The library exposes several key components:

- **Storage Traits**: `Storage` and `MetadataStorage` for implementing custom backends
- **Memory Storage**: `MemoryStorage` - ready-to-use in-memory implementation
- **Query Engine**: `SimpleQueryEngine` for parsing and executing basic PromQL queries
- **Label Matchers**: Extensible label filtering (`EqualMatcher`, `RegexMatcher`, etc.)
- **HTTP Layer**: `build_router()` and `AppState` for creating HTTP servers
- **Fixtures**: `FixtureBook` for loading predefined responses

## License

[MIT](./LICENSE)
