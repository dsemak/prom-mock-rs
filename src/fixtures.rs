//! Fixture definitions for predefined API responses and route matching.

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::timeutil::{resolve_relative, ResolvedParam};

/// Errors that can occur when loading or processing fixtures.
#[derive(Debug, Error)]
pub enum FixtureError {
    /// I/O error while reading fixture file.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// YAML parsing error.
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// A collection of fixture routes and their default settings.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct FixtureBook {
    /// Schema version of the fixture file.
    pub version: Option<u8>,
    /// Default settings for responses.
    pub defaults: Option<Defaults>,
    /// List of route matchers and their responses.
    pub routes: Vec<Route>,
}

/// Default settings for fixture responses.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Defaults {
    /// Default status ("success" by default).
    pub status: Option<String>,
    /// Clock anchor for relative time resolution (ISO/now).
    pub clock_anchor: Option<String>,
}

/// A route definition with matcher and response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Route {
    /// Request matcher criteria.
    #[serde(rename = "match")]
    pub matcher: Matcher,
    /// Response to return when matched.
    pub respond: Respond,
}

/// Request matching criteria for a fixture route.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Matcher {
    /// API path (`/api/v1/query` or `/api/v1/query_range`).
    pub path: String,
    /// `PromQL` query string.
    pub query: Option<String>,
    /// Start time for `query_range`.
    pub start: Option<String>,
    /// End time for `query_range`.
    pub end: Option<String>,
    /// Step interval for `query_range`.
    pub step: Option<String>,
}

/// Response data for a matched fixture route.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Respond {
    /// Response status (success/error).
    pub status: Option<String>,
    /// Response data in Prometheus format.
    pub data: serde_json::Value,
    /// Warning messages.
    pub warnings: Option<Vec<String>>,
    /// Error type for error responses.
    #[serde(rename = "errorType")]
    pub error_type: Option<String>,
    /// Error message for error responses.
    pub error: Option<String>,
}

impl FixtureBook {
    /// Load fixtures from a YAML file.
    ///
    /// # Parameters
    ///
    /// - `path` - Path to the YAML fixtures file
    ///
    /// # Returns
    ///
    /// Returns `Ok(FixtureBook)` on success, or `FixtureError` if the file cannot be read or parsed.
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, FixtureError> {
        let txt = fs::read_to_string(path)?;
        let mut book: Self = serde_yaml::from_str(&txt)?;
        // defaults.status defaults to success
        if book.defaults.is_none() {
            book.defaults = Some(Defaults { status: Some("success".into()), clock_anchor: None });
        } else if let Some(defaults) = &book.defaults {
            if defaults.status.is_none() {
                if let Some(defaults_mut) = &mut book.defaults {
                    defaults_mut.status = Some("success".into());
                }
            }
        }
        Ok(book)
    }

    /// Find a matching fixture route for the given request parameters.
    ///
    /// # Parameters
    ///
    /// - `path` - API path like "/api/v1/query" or "/`api/v1/query_range`"
    /// - `params` - Query parameters including `PromQL` query and time range
    /// - `now` - Optional fixed time for relative time resolution
    ///
    /// # Returns
    ///
    /// Returns `Some(Respond)` if a matching route is found, `None` otherwise.
    pub fn find_match(
        &self,
        path: &str,
        params: &QueryParams,
        now: Option<time::OffsetDateTime>,
    ) -> Option<&Respond> {
        self.routes.iter().find_map(|r| {
            if r.matcher.path != path {
                return None;
            }

            // query must match if specified
            if let Some(q) = &r.matcher.query {
                if &params.query != q {
                    return None;
                }
            }

            // For query_range - compare start/end/step, support relative time
            if path.ends_with("/query_range") {
                let (Some(start), Some(end), Some(step)) =
                    (&params.start, &params.end, &params.step)
                else {
                    return None;
                };

                // Fixture can contain absolute values or relative (now-15m)
                if let Some(expect_start) = &r.matcher.start {
                    if !param_equal(expect_start, start, now) {
                        return None;
                    }
                }
                if let Some(expect_end) = &r.matcher.end {
                    if !param_equal(expect_end, end, now) {
                        return None;
                    }
                }
                if let Some(expect_step) = &r.matcher.step {
                    if expect_step != step {
                        return None;
                    }
                }
            }

            Some(&r.respond)
        })
    }

    /// Get the effective status for a response, using defaults if not specified.
    ///
    /// # Parameters
    ///
    /// - `resp` - Response object to get status from
    ///
    /// # Returns
    ///
    /// Returns the status string, defaulting to "success" if not specified.
    pub fn effective_status<'a>(&'a self, resp: &'a Respond) -> &'a str {
        resp.status
            .as_deref()
            .or_else(|| self.defaults.as_ref().and_then(|d| d.status.as_deref()))
            .unwrap_or("success")
    }
}

#[allow(clippy::unnested_or_patterns)]
fn param_equal(expect: &str, got: &str, now: Option<time::OffsetDateTime>) -> bool {
    match (resolve_relative(expect, now), resolve_relative(got, now)) {
        // All value comparisons - resolved parameters can compare to each other and raw to raw
        (ResolvedParam::Absolute(e), ResolvedParam::Absolute(g))
        | (ResolvedParam::Relative(e), ResolvedParam::Relative(g))
        | (ResolvedParam::Relative(e), ResolvedParam::Absolute(g))
        | (ResolvedParam::Absolute(e), ResolvedParam::Relative(g))
        | (ResolvedParam::Raw(e), ResolvedParam::Raw(g)) => e == g,
        // Raw vs resolved types don't match
        (ResolvedParam::Raw(_), ResolvedParam::Absolute(_) | ResolvedParam::Relative(_))
        | (ResolvedParam::Absolute(_) | ResolvedParam::Relative(_), ResolvedParam::Raw(_)) => false,
    }
}

/// Query parameters in unified form.
pub struct QueryParams {
    pub query: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub step: Option<String>,
}
