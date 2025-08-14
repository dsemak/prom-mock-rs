//! Remote Write Protocol implementation.
//!
//! This module handles Prometheus remote write requests, parsing protobuf
//! data and storing it in the in-memory time series database.

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use prost::Message;
use tracing::{debug, warn};

use crate::http::handlers::health::maybe_latency_and_error;
use crate::http::state::AppState;
use crate::storage::{
    FullStorage, Label as StorageLabel, Sample as StorageSample, TimeSeries as StorageTimeSeries,
};

// Include the generated protobuf code
include!(concat!(env!("OUT_DIR"), "/prometheus.rs"));

/// Handle remote write requests from Prometheus or compatible agents.
///
/// # Parameters
///
/// - `state` - Application state with storage and simulation settings
/// - `headers` - HTTP headers, checked for content encoding
/// - `body` - Request body containing protobuf-encoded metrics
///
/// # Returns
///
/// Returns HTTP 204 on success, or error status with message on failure.
pub async fn remote_write(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Apply latency and error simulation
    if let Err(code) = maybe_latency_and_error(&state).await {
        return (code, "simulated failure").into_response();
    }

    handle_remote_write_impl(State(state.query.storage.clone()), &headers, body).into_response()
}

/// Internal implementation of remote write handling.
///
/// # Parameters
///
/// - `storage` - Shared reference to storage implementation for persisting metrics
/// - `headers` - HTTP headers, checked for content encoding
/// - `body` - Request body containing protobuf-encoded metrics
///
/// # Returns
///
/// Returns HTTP 204 on success, or error status with message on failure.
fn handle_remote_write_impl(
    State(storage): State<Arc<dyn FullStorage>>,
    headers: &HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Check Content-Encoding for snappy compression
    let is_snappy = headers
        .get("content-encoding")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("snappy"));

    // For now, we don't handle snappy compression - would need snappy crate
    if is_snappy {
        warn!("snappy compression not supported yet");
        return (StatusCode::BAD_REQUEST, "snappy compression not supported").into_response();
    }

    // Decode protobuf
    let write_request = match WriteRequest::decode(body) {
        Ok(req) => req,
        Err(e) => {
            warn!("failed to decode remote write request: {}", e);
            return (StatusCode::BAD_REQUEST, "invalid protobuf").into_response();
        }
    };

    debug!("received remote write request with {} series", write_request.timeseries.len());

    // Convert protobuf to our internal format and store
    for proto_ts in write_request.timeseries {
        let labels: Vec<StorageLabel> =
            proto_ts.labels.into_iter().map(|l| StorageLabel::new(l.name, l.value)).collect();

        let mut ts = StorageTimeSeries::new(labels);

        for proto_sample in proto_ts.samples {
            ts.add_sample(StorageSample::new(proto_sample.timestamp, proto_sample.value));
        }

        storage.add_series(ts);
    }

    // Return 204 No Content on success (standard for remote write)
    StatusCode::NO_CONTENT.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test protobuf encoding and decoding of remote write messages.
    #[test]
    fn test_protobuf_encoding() {
        let write_request = WriteRequest {
            timeseries: vec![TimeSeries {
                labels: vec![
                    Label { name: "__name__".to_string(), value: "test_metric".to_string() },
                    Label { name: "job".to_string(), value: "test".to_string() },
                ],
                samples: vec![Sample {
                    timestamp: 1640995200000, // 2022-01-01 00:00:00 UTC
                    value: 42.0,
                }],
            }],
        };

        let mut buf = Vec::new();
        write_request.encode(&mut buf).expect("valid protobuf message");

        // Decode back
        let decoded = WriteRequest::decode(buf.as_slice()).expect("just encoded valid data");
        assert_eq!(decoded.timeseries.len(), 1);
        assert_eq!(decoded.timeseries[0].samples[0].value, 42.0);
    }
}
