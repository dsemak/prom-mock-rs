//! Label matching implementations for filtering time series.
//!
//! This module provides various label matching strategies that can be used
//! to filter time series data based on label criteria. It follows the Open/Closed
//! Principle by using traits, allowing new matcher types to be added without
//! modifying existing code.

use regex::Regex;

use crate::storage::Label;

/// Label matcher trait for extensible label filtering operations.
///
/// This trait allows implementing custom label matching logic while maintaining
/// compatibility with the storage system. New matcher types can be added without
/// modifying existing code (Open/Closed Principle).
pub trait LabelMatcher: Send + Sync + std::fmt::Debug {
    /// Check if this matcher matches the given labels.
    ///
    /// # Parameters
    ///
    /// - `labels` - Array of labels to test against
    ///
    /// # Returns
    ///
    /// Returns `true` if the matcher matches any label in the array.
    fn matches(&self, labels: &[Label]) -> bool;

    /// Get the label name this matcher operates on.
    ///
    /// # Returns
    ///
    /// Returns the name of the label this matcher filters on.
    fn label_name(&self) -> &str;
}

/// Equality matcher for exact label value matching.
#[derive(Debug, Clone)]
pub struct EqualMatcher {
    pub name: String,
    pub value: String,
}

impl EqualMatcher {
    /// Create a new equality matcher.
    ///
    /// # Parameters
    ///
    /// - `name` - Label name to match
    /// - `value` - Exact value to match
    ///
    /// # Returns
    ///
    /// Returns a new `EqualMatcher` instance.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self { name: name.into(), value: value.into() }
    }
}

impl LabelMatcher for EqualMatcher {
    fn matches(&self, labels: &[Label]) -> bool {
        labels.iter().any(|l| l.name == self.name && l.value == self.value)
    }

    fn label_name(&self) -> &str {
        &self.name
    }
}

/// Not-equality matcher for excluding specific label values.
#[derive(Debug, Clone)]
pub struct NotEqualMatcher {
    pub name: String,
    pub value: String,
}

impl NotEqualMatcher {
    /// Create a new not-equality matcher.
    ///
    /// # Parameters
    ///
    /// - `name` - Label name to match
    /// - `value` - Value that should not match
    ///
    /// # Returns
    ///
    /// Returns a new `NotEqualMatcher` instance.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self { name: name.into(), value: value.into() }
    }
}

impl LabelMatcher for NotEqualMatcher {
    fn matches(&self, labels: &[Label]) -> bool {
        !labels.iter().any(|l| l.name == self.name && l.value == self.value)
    }

    fn label_name(&self) -> &str {
        &self.name
    }
}

/// Regex matcher for pattern-based label value matching.
#[derive(Debug)]
pub struct RegexMatcher {
    pub name: String,
    pub pattern: Regex,
}

impl RegexMatcher {
    /// Create a new regex matcher.
    ///
    /// # Parameters
    ///
    /// - `name` - Label name to match
    /// - `pattern` - Compiled regex pattern
    ///
    /// # Returns
    ///
    /// Returns a new `RegexMatcher` instance.
    pub fn new(name: impl Into<String>, pattern: Regex) -> Self {
        Self { name: name.into(), pattern }
    }
}

impl LabelMatcher for RegexMatcher {
    fn matches(&self, labels: &[Label]) -> bool {
        labels.iter().any(|l| l.name == self.name && self.pattern.is_match(&l.value))
    }

    fn label_name(&self) -> &str {
        &self.name
    }
}

/// Not-regex matcher for excluding pattern-based label values.
#[derive(Debug)]
pub struct NotRegexMatcher {
    pub name: String,
    pub pattern: Regex,
}

impl NotRegexMatcher {
    /// Create a new not-regex matcher.
    ///
    /// # Parameters
    ///
    /// - `name` - Label name to match
    /// - `pattern` - Compiled regex pattern that should not match
    ///
    /// # Returns
    ///
    /// Returns a new `NotRegexMatcher` instance.
    pub fn new(name: impl Into<String>, pattern: regex::Regex) -> Self {
        Self { name: name.into(), pattern }
    }
}

impl LabelMatcher for NotRegexMatcher {
    fn matches(&self, labels: &[Label]) -> bool {
        !labels.iter().any(|l| l.name == self.name && self.pattern.is_match(&l.value))
    }

    fn label_name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test equality and inequality matchers with various label combinations.
    #[test]
    fn test_basic_matchers() {
        let labels = vec![
            Label::new("__name__", "http_requests_total"),
            Label::new("job", "api"),
            Label::new("method", "GET"),
        ];

        // Test equality matcher
        let matcher = EqualMatcher::new("job", "api");
        assert!(matcher.matches(&labels));

        let matcher = EqualMatcher::new("job", "web");
        assert!(!matcher.matches(&labels));

        // Test not-equality matcher
        let matcher = NotEqualMatcher::new("job", "web");
        assert!(matcher.matches(&labels));

        let matcher = NotEqualMatcher::new("job", "api");
        assert!(!matcher.matches(&labels));
    }

    /// Test regex matchers with pattern matching.
    #[test]
    fn test_regex_matchers() {
        let labels = vec![Label::new("service", "web-frontend"), Label::new("version", "v1.2.3")];

        // Test regex matcher
        let pattern = Regex::new(r"^web.*").expect("valid regex");
        let matcher = RegexMatcher::new("service", pattern);
        assert!(matcher.matches(&labels));

        // Test not-regex matcher
        let pattern = Regex::new(r"^api.*").expect("valid regex");
        let matcher = NotRegexMatcher::new("service", pattern);
        assert!(matcher.matches(&labels));
    }
}
