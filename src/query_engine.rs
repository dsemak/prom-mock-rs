//! Simple query engine for basic metric selectors without full `PromQL`.
//!
//! This module provides a basic query parser and executor that supports
//! simple metric selectors like `metric{label="value"}` without requiring
//! a full `PromQL` implementation.

use std::io;
use std::sync::Arc;

use regex::Regex;

use crate::matchers::{EqualMatcher, LabelMatcher, NotEqualMatcher, NotRegexMatcher, RegexMatcher};
use crate::storage::Storage;

/// Simple query parser for basic selectors like: metric{a="b",c!="d",e=~"regex"}.
#[derive(Clone)]
pub struct SimpleQueryEngine {
    storage: Arc<dyn Storage>,
}

impl SimpleQueryEngine {
    /// Create a new query engine with the given storage backend.
    ///
    /// # Parameters
    ///
    /// - `storage` - Shared reference to any storage implementation
    ///
    /// # Returns
    ///
    /// Returns a new `SimpleQueryEngine` instance.
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Parse and execute a simple metric selector query
    pub fn query(&self, query: &str, start: i64, end: i64) -> io::Result<QueryResult> {
        let selector = Self::parse_selector(query)?;
        let series = self.storage.query_series(&selector.matchers);

        let mut result_series = Vec::new();
        for ts in series {
            let samples = ts.samples_in_range(start, end);
            if !samples.is_empty() {
                result_series.push(QueryResultSeries {
                    labels: ts.labels.clone(),
                    samples: samples.into_iter().cloned().collect(),
                });
            }
        }

        Ok(QueryResult { series: result_series })
    }

    /// Parse a simple selector like: metric{a="b",c!="d",e=~"regex"}
    fn parse_selector(query: &str) -> io::Result<MetricSelector> {
        let query = query.trim();

        // Split metric name and labels
        let (metric_name, labels_part) = query.find('{').map_or((Some(query), None), |brace_pos| {
            let name = query[..brace_pos].trim();
            let labels = &query[brace_pos..];
            (Some(name), Some(labels))
        });

        let mut matchers: Vec<Arc<dyn LabelMatcher>> = Vec::new();

        // Add metric name matcher if present
        if let Some(name) = metric_name {
            if !name.is_empty() {
                matchers.push(Arc::new(EqualMatcher::new("__name__", name)));
            }
        }

        // Parse label matchers if present
        if let Some(labels) = labels_part {
            let label_matchers = Self::parse_label_matchers(labels)?;
            matchers.extend(label_matchers);
        }

        Ok(MetricSelector { matchers })
    }

    /// Parse label matchers from string like: {a="b",c!="d",e=~"regex"}
    fn parse_label_matchers(labels_str: &str) -> io::Result<Vec<Arc<dyn LabelMatcher>>> {
        let labels_str = labels_str.trim();
        if !labels_str.starts_with('{') || !labels_str.ends_with('}') {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid label syntax"));
        }

        let inner = &labels_str[1..labels_str.len() - 1];
        if inner.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut matchers = Vec::new();
        let parts = Self::split_label_expressions(inner);

        for part in parts {
            let matcher = Self::parse_single_label_matcher(&part)?;
            matchers.push(matcher);
        }

        Ok(matchers)
    }

    /// Split label expressions by comma, handling quoted strings
    fn split_label_expressions(input: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut escape_next = false;

        for ch in input.chars() {
            if escape_next {
                current.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' => {
                    escape_next = true;
                    current.push(ch);
                }
                '"' => {
                    in_quotes = !in_quotes;
                    current.push(ch);
                }
                ',' if !in_quotes => {
                    parts.push(current.trim().to_string());
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.trim().is_empty() {
            parts.push(current.trim().to_string());
        }

        parts
    }

    /// Parse a single label matcher like: a="b" or c!="d" or e=~"regex"
    fn parse_single_label_matcher(expr: &str) -> io::Result<Arc<dyn LabelMatcher>> {
        let expr = expr.trim();

        // Find operator
        if let Some(pos) = expr.find("!=") {
            let name = expr[..pos].trim().to_string();
            let value = Self::parse_quoted_value(&expr[pos + 2..])?;
            return Ok(Arc::new(NotEqualMatcher::new(name, value)));
        }

        if let Some(pos) = expr.find("!~") {
            let name = expr[..pos].trim().to_string();
            let pattern_str = Self::parse_quoted_value(&expr[pos + 2..])?;
            let pattern = Regex::new(&pattern_str).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidInput, format!("invalid regex: {e}"))
            })?;
            return Ok(Arc::new(NotRegexMatcher::new(name, pattern)));
        }

        if let Some(pos) = expr.find("=~") {
            let name = expr[..pos].trim().to_string();
            let pattern_str = Self::parse_quoted_value(&expr[pos + 2..])?;
            let pattern = Regex::new(&pattern_str).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidInput, format!("invalid regex: {e}"))
            })?;
            return Ok(Arc::new(RegexMatcher::new(name, pattern)));
        }

        if let Some(pos) = expr.find('=') {
            let name = expr[..pos].trim().to_string();
            let value = Self::parse_quoted_value(&expr[pos + 1..])?;
            return Ok(Arc::new(EqualMatcher::new(name, value)));
        }

        Err(io::Error::new(io::ErrorKind::InvalidInput, format!("invalid label matcher: {expr}")))
    }

    /// Parse quoted value, removing quotes
    fn parse_quoted_value(input: &str) -> io::Result<String> {
        let input = input.trim();
        if input.starts_with('"') && input.ends_with('"') && input.len() >= 2 {
            Ok(input[1..input.len() - 1].to_string())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("expected quoted value: {input}"),
            ))
        }
    }
}

#[derive(Debug)]
struct MetricSelector {
    matchers: Vec<Arc<dyn LabelMatcher>>,
}

/// Query result containing time series.
#[derive(Debug)]
pub struct QueryResult {
    pub series: Vec<QueryResultSeries>,
}

#[derive(Debug)]
pub struct QueryResultSeries {
    pub labels: Vec<crate::storage::Label>,
    pub samples: Vec<crate::storage::Sample>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Label, MemoryStorage, Sample, TimeSeries};

    /// Test parsing of simple metric selectors with and without labels.
    #[test]
    fn test_parse_simple_selector() {
        let _engine = SimpleQueryEngine::new(Arc::new(MemoryStorage::new()));

        // Test simple metric name
        let selector = SimpleQueryEngine::parse_selector("up").expect("valid syntax");
        assert_eq!(selector.matchers.len(), 1);

        // Test with labels
        let selector =
            SimpleQueryEngine::parse_selector(r#"http_requests{job="api",method!="POST"}"#)
                .expect("valid syntax");
        assert_eq!(selector.matchers.len(), 3); // __name__, job, method
    }

    /// Test splitting of label expressions with proper quote handling.
    #[test]
    fn test_split_label_expressions() {
        let parts = SimpleQueryEngine::split_label_expressions(r#"a="b",c!="d",e=~"regex""#);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], r#"a="b""#);
        assert_eq!(parts[1], r#"c!="d""#);
        assert_eq!(parts[2], r#"e=~"regex""#);
    }

    /// Test parsing of individual label matchers for equality and inequality operations.
    #[test]
    fn test_parse_label_matcher() {
        let matcher =
            SimpleQueryEngine::parse_single_label_matcher(r#"job="api""#).expect("valid syntax");
        assert_eq!(matcher.label_name(), "job");

        let labels = vec![crate::storage::Label::new("job", "api")];
        assert!(matcher.matches(&labels));

        let matcher = SimpleQueryEngine::parse_single_label_matcher(r#"method!="POST""#)
            .expect("valid syntax");
        assert_eq!(matcher.label_name(), "method");

        let labels = vec![crate::storage::Label::new("method", "GET")];
        assert!(matcher.matches(&labels));
    }

    /// Test end-to-end query functionality with in-memory storage.
    #[test]
    fn test_query_with_storage() {
        let storage = Arc::new(MemoryStorage::new());
        let engine = SimpleQueryEngine::new(storage.clone());

        // Add test data
        let labels = vec![
            Label::new("__name__", "http_requests"),
            Label::new("job", "api"),
            Label::new("method", "GET"),
        ];
        let mut ts = TimeSeries::new(labels);
        ts.add_sample(Sample::new(1000, 10.0));
        ts.add_sample(Sample::new(2000, 20.0));
        storage.add_series(ts);

        // Query
        let result = engine.query(r#"http_requests{job="api"}"#, 0, 3000).expect("valid query");
        assert_eq!(result.series.len(), 1);
        assert_eq!(result.series[0].samples.len(), 2);
    }
}
