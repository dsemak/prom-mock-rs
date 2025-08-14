//! Fixture-based handlers for mocking Prometheus API responses.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::fixtures::QueryParams as FQueryParams;
use crate::http::handlers::health::maybe_latency_and_error;
use crate::http::state::AppState;
use crate::http::types::{PromApiResponse, QueryParams, QueryRangeParams};

/// Handle instant query requests using fixtures.
///
/// # Parameters
///
/// - `state` - Application state containing fixture data
/// - `params` - Query parameters from request
///
/// # Returns
///
/// Returns fixture response if matching fixture is found, otherwise 404.
pub async fn query(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    if let Err(code) = maybe_latency_and_error(&state).await {
        return (code, "simulated failure").into_response();
    }

    let qp = FQueryParams { query: params.query.clone(), start: None, end: None, step: None };

    if let Some(resp) = state.mock.fixtures.find_match("/api/v1/query", &qp, state.mock.fixed_now) {
        let status = state.mock.fixtures.effective_status(resp);
        return (
            StatusCode::OK,
            Json(PromApiResponse {
                status,
                data: Some(resp.data.clone()),
                warnings: resp.warnings.as_ref(),
                error_type: resp.error_type.as_ref(),
                error: resp.error.as_ref(),
            }),
        )
            .into_response();
    }

    // No match found - return 404 in Prometheus style
    (
        StatusCode::NOT_FOUND,
        Json(PromApiResponse {
            status: "error",
            data: None,
            warnings: None,
            error_type: Some(&"not_found".to_string()),
            error: Some(&"no fixture matched".to_string()),
        }),
    )
        .into_response()
}

/// Handle query range requests using fixtures.
///
/// # Parameters
///
/// - `state` - Application state containing fixture data
/// - `params` - Query range parameters from request
///
/// # Returns
///
/// Returns fixture response if matching fixture is found, otherwise 404.
pub async fn query_range(
    State(state): State<AppState>,
    Query(params): Query<QueryRangeParams>,
) -> impl IntoResponse {
    if let Err(code) = maybe_latency_and_error(&state).await {
        return (code, "simulated failure").into_response();
    }

    // Normalize input: if relative values came in and we have fixed_now - resolve them.
    let start = stringify_resolved(&params.start, state.mock.fixed_now);
    let end = stringify_resolved(&params.end, state.mock.fixed_now);

    let qp = FQueryParams {
        query: params.query.clone(),
        start: Some(start),
        end: Some(end),
        step: Some(params.step.clone()),
    };

    if let Some(resp) =
        state.mock.fixtures.find_match("/api/v1/query_range", &qp, state.mock.fixed_now)
    {
        let status = state.mock.fixtures.effective_status(resp);
        return (
            StatusCode::OK,
            Json(PromApiResponse {
                status,
                data: Some(resp.data.clone()),
                warnings: resp.warnings.as_ref(),
                error_type: resp.error_type.as_ref(),
                error: resp.error.as_ref(),
            }),
        )
            .into_response();
    }

    (
        StatusCode::NOT_FOUND,
        Json(PromApiResponse {
            status: "error",
            data: None,
            warnings: None,
            error_type: Some(&"not_found".to_string()),
            error: Some(&"no fixture matched".to_string()),
        }),
    )
        .into_response()
}

/// Convert relative time parameters to string format.
fn stringify_resolved(input: &str, now: Option<time::OffsetDateTime>) -> String {
    match crate::timeutil::resolve_relative(input, now) {
        crate::timeutil::ResolvedParam::Absolute(s)
        | crate::timeutil::ResolvedParam::Relative(s)
        | crate::timeutil::ResolvedParam::Raw(s) => s,
    }
}
