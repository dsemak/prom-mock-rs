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

    /// Test edge cases with empty and nonexistent labels.
    #[test]
    fn test_edge_cases() {
        let empty_labels: Vec<Label> = vec![];
        let labels_with_empty = vec![Label::new("", ""), Label::new("key", "")];

        // Test with empty labels array
        let matcher = EqualMatcher::new("job", "api");
        assert!(!matcher.matches(&empty_labels));

        let matcher = NotEqualMatcher::new("job", "api");
        assert!(matcher.matches(&empty_labels)); // Should match since no label equals "job=api"

        // Test with empty label names/values
        let matcher = EqualMatcher::new("", "");
        assert!(matcher.matches(&labels_with_empty));

        let matcher = EqualMatcher::new("key", "");
        assert!(matcher.matches(&labels_with_empty));

        // Test nonexistent label
        let normal_labels = vec![Label::new("job", "api")];
        let matcher = EqualMatcher::new("nonexistent", "value");
        assert!(!matcher.matches(&normal_labels));
    }

    /// Test label_name method for all matcher types.
    #[test]
    fn test_label_name_methods() {
        let equal_matcher = EqualMatcher::new("test_label", "value");
        assert_eq!(equal_matcher.label_name(), "test_label");

        let not_equal_matcher = NotEqualMatcher::new("another_label", "value");
        assert_eq!(not_equal_matcher.label_name(), "another_label");

        let pattern = Regex::new(r".*").expect("valid regex");
        let regex_matcher = RegexMatcher::new("regex_label", pattern);
        assert_eq!(regex_matcher.label_name(), "regex_label");

        let pattern = Regex::new(r".*").expect("valid regex");
        let not_regex_matcher = NotRegexMatcher::new("not_regex_label", pattern);
        assert_eq!(not_regex_matcher.label_name(), "not_regex_label");
    }

    /// Test complex regex patterns and special cases.
    #[test]
    fn test_complex_regex_patterns() {
        let labels = vec![
            Label::new("version", "v1.2.3"),
            Label::new("environment", "production"),
            Label::new("special", "test@domain.com"),
        ];

        // Test version pattern
        let version_pattern = Regex::new(r"^v\d+\.\d+\.\d+$").expect("valid regex");
        let matcher = RegexMatcher::new("version", version_pattern);
        assert!(matcher.matches(&labels));

        // Test email pattern
        let email_pattern = Regex::new(r".*@.*\.com$").expect("valid regex");
        let matcher = RegexMatcher::new("special", email_pattern);
        assert!(matcher.matches(&labels));

        // Test not-regex with non-matching pattern
        let non_matching_pattern = Regex::new(r"^staging.*").expect("valid regex");
        let matcher = NotRegexMatcher::new("environment", non_matching_pattern);
        assert!(matcher.matches(&labels)); // Should match since environment != "staging*"

        // Test not-regex with matching pattern
        let matching_pattern = Regex::new(r"^prod.*").expect("valid regex");
        let matcher = NotRegexMatcher::new("environment", matching_pattern);
        assert!(!matcher.matches(&labels)); // Should not match since environment = "production"
    }

    /// Test multiple labels with same name (edge case).
    #[test]
    fn test_duplicate_label_names() {
        let labels = vec![
            Label::new("tag", "first"),
            Label::new("tag", "second"),
            Label::new("other", "value"),
        ];

        // Equal matcher should match if any label with the name matches
        let matcher = EqualMatcher::new("tag", "first");
        assert!(matcher.matches(&labels));

        let matcher = EqualMatcher::new("tag", "second");
        assert!(matcher.matches(&labels));

        let matcher = EqualMatcher::new("tag", "third");
        assert!(!matcher.matches(&labels));

        // Not-equal matcher should not match if ANY label with the name matches
        let matcher = NotEqualMatcher::new("tag", "first");
        assert!(!matcher.matches(&labels)); // Should not match because one "tag" equals "first"

        let matcher = NotEqualMatcher::new("tag", "third");
        assert!(matcher.matches(&labels)); // Should match because no "tag" equals "third"
    }
}
