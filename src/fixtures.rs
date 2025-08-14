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
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Defaults {
    /// Default status ("success" by default).
    pub status: Option<String>,
    /// Clock anchor for relative time resolution (ISO/now).
    pub clock_anchor: Option<String>,
}

/// A route definition with matcher and response.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Route {
    /// Request matcher criteria.
    #[serde(rename = "match")]
    pub matcher: Matcher,
    /// Response to return when matched.
    pub respond: Respond,
}

/// Request matching criteria for a fixture route.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
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

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::NamedTempFile;
    use time::macros::datetime;

    use super::*;

    /// Test basic FixtureBook creation and default values.
    #[test]
    fn test_fixture_book_defaults() {
        let book = FixtureBook::default();
        assert_eq!(book.version, None);
        assert_eq!(book.defaults, None);
        assert!(book.routes.is_empty());

        // Test with defaults
        let book = FixtureBook {
            version: Some(1),
            defaults: Some(Defaults {
                status: Some("success".to_string()),
                clock_anchor: Some("now".to_string()),
            }),
            routes: vec![],
        };
        assert_eq!(book.version, Some(1));
        assert!(book.defaults.is_some());
    }

    /// Test loading FixtureBook from YAML with various configurations.
    #[test]
    fn test_load_from_yaml() {
        // Test minimal YAML
        let yaml_content = r#"
version: 1
routes: []
"#;
        let temp_file = NamedTempFile::new().expect("create temp file");
        fs::write(&temp_file, yaml_content).expect("write temp file");

        let book = FixtureBook::load_from_path(&temp_file).expect("load fixture book");
        assert_eq!(book.version, Some(1));
        assert!(book.routes.is_empty());
        // Should have default status
        assert_eq!(book.defaults.as_ref().unwrap().status.as_ref().unwrap(), "success");

        // Test with explicit defaults
        let yaml_content = r#"
version: 1
defaults:
  status: "error"
  clock_anchor: "2022-01-01T00:00:00Z"
routes:
  - match:
      path: "/api/v1/query"
      query: "up"
    respond:
      data: {"resultType": "vector", "result": []}
"#;
        let temp_file = NamedTempFile::new().expect("create temp file");
        fs::write(&temp_file, yaml_content).expect("write temp file");

        let book = FixtureBook::load_from_path(&temp_file).expect("load fixture book");
        assert_eq!(book.version, Some(1));
        assert_eq!(book.defaults.as_ref().unwrap().status.as_ref().unwrap(), "error");
        assert_eq!(book.routes.len(), 1);
        assert_eq!(book.routes[0].matcher.path, "/api/v1/query");
        assert_eq!(book.routes[0].matcher.query.as_ref().unwrap(), "up");
    }

    /// Test invalid YAML handling.
    #[test]
    fn test_load_invalid_yaml() {
        let yaml_content = "invalid: yaml: content: [";
        let temp_file = NamedTempFile::new().expect("create temp file");
        fs::write(&temp_file, yaml_content).expect("write temp file");

        let result = FixtureBook::load_from_path(&temp_file);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FixtureError::Yaml(_)));
    }

    /// Test file not found handling.
    #[test]
    fn test_load_nonexistent_file() {
        let result = FixtureBook::load_from_path("/nonexistent/file.yaml");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FixtureError::Io(_)));
    }

    /// Test finding matches for simple queries.
    #[test]
    fn test_find_match_simple_query() {
        let book = FixtureBook {
            version: Some(1),
            defaults: Some(Defaults { status: Some("success".to_string()), clock_anchor: None }),
            routes: vec![
                Route {
                    matcher: Matcher {
                        path: "/api/v1/query".to_string(),
                        query: Some("up".to_string()),
                        start: None,
                        end: None,
                        step: None,
                    },
                    respond: Respond {
                        status: None,
                        data: json!({"resultType": "vector", "result": []}),
                        warnings: None,
                        error_type: None,
                        error: None,
                    },
                },
                Route {
                    matcher: Matcher {
                        path: "/api/v1/query".to_string(),
                        query: Some("cpu_usage".to_string()),
                        start: None,
                        end: None,
                        step: None,
                    },
                    respond: Respond {
                        status: Some("error".to_string()),
                        data: json!({}),
                        warnings: None,
                        error_type: Some("execution".to_string()),
                        error: Some("query failed".to_string()),
                    },
                },
            ],
        };

        // Test matching query
        let params = QueryParams { query: "up".to_string(), start: None, end: None, step: None };
        let result = book.find_match("/api/v1/query", &params, None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().data["resultType"], "vector");

        // Test different query
        let params =
            QueryParams { query: "cpu_usage".to_string(), start: None, end: None, step: None };
        let result = book.find_match("/api/v1/query", &params, None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().status.as_ref().unwrap(), "error");

        // Test non-matching query
        let params =
            QueryParams { query: "memory_usage".to_string(), start: None, end: None, step: None };
        let result = book.find_match("/api/v1/query", &params, None);
        assert!(result.is_none());

        // Test wrong path
        let params = QueryParams { query: "up".to_string(), start: None, end: None, step: None };
        let result = book.find_match("/api/v1/query_range", &params, None);
        assert!(result.is_none());
    }

    /// Test finding matches for query_range requests.
    #[test]
    fn test_find_match_query_range() {
        let book = FixtureBook {
            version: Some(1),
            defaults: None,
            routes: vec![Route {
                matcher: Matcher {
                    path: "/api/v1/query_range".to_string(),
                    query: Some("up".to_string()),
                    start: Some("now-1h".to_string()),
                    end: Some("now".to_string()),
                    step: Some("5m".to_string()),
                },
                respond: Respond {
                    status: None,
                    data: json!({"resultType": "matrix", "result": []}),
                    warnings: None,
                    error_type: None,
                    error: None,
                },
            }],
        };

        let fixed_time = datetime!(2022-01-01 12:00:00 UTC);

        // Test matching query_range
        let params = QueryParams {
            query: "up".to_string(),
            start: Some("now-1h".to_string()),
            end: Some("now".to_string()),
            step: Some("5m".to_string()),
        };
        let result = book.find_match("/api/v1/query_range", &params, Some(fixed_time));
        assert!(result.is_some());
        assert_eq!(result.unwrap().data["resultType"], "matrix");

        // Test missing required parameters for query_range
        let params = QueryParams {
            query: "up".to_string(),
            start: None,
            end: Some("now".to_string()),
            step: Some("5m".to_string()),
        };
        let result = book.find_match("/api/v1/query_range", &params, Some(fixed_time));
        assert!(result.is_none());

        // Test different step value
        let params = QueryParams {
            query: "up".to_string(),
            start: Some("now-1h".to_string()),
            end: Some("now".to_string()),
            step: Some("1m".to_string()),
        };
        let result = book.find_match("/api/v1/query_range", &params, Some(fixed_time));
        assert!(result.is_none());
    }

    /// Test effective_status method with defaults.
    #[test]
    fn test_effective_status() {
        let book = FixtureBook {
            version: None,
            defaults: Some(Defaults {
                status: Some("default_success".to_string()),
                clock_anchor: None,
            }),
            routes: vec![],
        };

        // Response with explicit status
        let respond = Respond {
            status: Some("custom_status".to_string()),
            data: json!({}),
            warnings: None,
            error_type: None,
            error: None,
        };
        assert_eq!(book.effective_status(&respond), "custom_status");

        // Response without explicit status (should use default)
        let respond = Respond {
            status: None,
            data: json!({}),
            warnings: None,
            error_type: None,
            error: None,
        };
        assert_eq!(book.effective_status(&respond), "default_success");

        // Book without defaults
        let book_no_defaults = FixtureBook::default();
        assert_eq!(book_no_defaults.effective_status(&respond), "success");
    }

    /// Test param_equal function with various time formats.
    #[test]
    fn test_param_equal() {
        let fixed_time = datetime!(2022-01-01 12:00:00 UTC);

        // Exact string matches
        assert!(param_equal("1640995200", "1640995200", None));
        assert!(!param_equal("1640995200", "1640995300", None));

        // Relative time comparisons
        assert!(param_equal("now", "now", Some(fixed_time)));
        assert!(param_equal("now-1h", "now-1h", Some(fixed_time)));
        assert!(!param_equal("now-1h", "now-2h", Some(fixed_time)));

        // Mixed absolute/relative should work if they resolve to same value
        let timestamp_1h_ago = "1641034800"; // fixed_time - 1 hour
        assert!(param_equal("now-1h", timestamp_1h_ago, Some(fixed_time)));

        // Raw vs resolved types should not match
        assert!(!param_equal("raw_string", "1640995200", Some(fixed_time)));
        assert!(!param_equal("1640995200", "raw_string", Some(fixed_time)));
    }
}
