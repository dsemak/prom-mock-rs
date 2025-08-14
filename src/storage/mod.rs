//! Time series storage implementations and abstractions.
//!
//! This module provides storage abstractions and implementations for time series data.
//! It includes traits for different storage capabilities and specific implementations
//! like in-memory storage.

pub mod memory;

// Re-export main implementations
pub use memory::MemoryStorage;

use std::sync::Arc;

use crate::matchers::LabelMatcher;

/// Storage abstraction for querying and storing time series data.
///
/// This trait provides the core operations needed for time series storage,
/// allowing different implementations (in-memory, disk-based, remote, etc.).
pub trait Storage: Send + Sync {
    /// Add or update a time series in storage.
    ///
    /// # Parameters
    ///
    /// - `ts` - Time series to store, samples will be merged if series already exists
    fn add_series(&self, ts: TimeSeries);

    /// Query series by label matchers.
    ///
    /// # Parameters
    ///
    /// - `matchers` - Array of label matchers to filter series
    ///
    /// # Returns
    ///
    /// Returns a vector of matching time series.
    fn query_series(&self, matchers: &[Arc<dyn LabelMatcher>]) -> Vec<TimeSeries>;
}

/// Metadata operations for storage introspection.
///
/// This trait provides operations to discover available labels and values
/// in the storage system, useful for query building and exploration.
pub trait MetadataStorage: Send + Sync {
    /// Get all label names from the storage.
    ///
    /// # Returns
    ///
    /// Returns a vector of all unique label names.
    fn label_names(&self) -> Vec<String>;

    /// Get all values for a specific label name.
    ///
    /// # Parameters
    ///
    ///  - `name` - Label name to get values for
    ///
    /// # Returns
    ///
    /// Returns a vector of all values for the label, or empty vector if label not found.
    fn label_values(&self, name: &str) -> Vec<String>;
}

/// Combined storage trait providing both data and metadata operations.
pub trait FullStorage: Storage + MetadataStorage {}

/// A metric label representing a name=value pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Label {
    pub name: String,
    pub value: String,
}

impl Label {
    /// Create a new label with the given name and value.
    ///
    /// # Parameters
    ///
    /// - `name` - Label name
    /// - `value` - Label value
    ///
    /// # Returns
    ///
    /// Returns a new `Label` instance.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self { name: name.into(), value: value.into() }
    }
}

/// A single metric sample with timestamp and value.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub timestamp: i64,
    pub value: f64,
}

impl Sample {
    /// Create a new sample with the given timestamp (milliseconds) and value.
    ///
    /// # Parameters
    ///
    /// - `timestamp` - Timestamp in milliseconds since Unix epoch
    /// - `value` - Metric value
    ///
    /// # Returns
    ///
    /// Returns a new `Sample` instance.
    pub const fn new(timestamp: i64, value: f64) -> Self {
        Self { timestamp, value }
    }
}

/// A time series containing labels and samples for a metric.
#[derive(Debug, Clone)]
pub struct TimeSeries {
    pub labels: Vec<Label>,
    pub samples: Vec<Sample>,
}

impl TimeSeries {
    /// Create a new time series with the given labels.
    ///
    /// # Parameters
    ///
    /// - `labels` - Vector of labels for this time series
    ///
    /// # Returns
    ///
    /// Returns a new `TimeSeries` instance with empty samples.
    pub const fn new(labels: Vec<Label>) -> Self {
        Self { labels, samples: Vec::new() }
    }

    /// Add a sample to this time series, maintaining sorted order by timestamp.
    ///
    /// # Parameters
    ///
    /// - `sample` - Sample to add, will replace existing sample at same timestamp
    pub fn add_sample(&mut self, sample: Sample) {
        // Keep samples sorted by timestamp
        match self.samples.binary_search_by_key(&sample.timestamp, |s| s.timestamp) {
            Ok(pos) => {
                // Replace existing sample at same timestamp
                self.samples[pos] = sample;
            }
            Err(pos) => {
                // Insert at correct position
                self.samples.insert(pos, sample);
            }
        }
    }

    /// Get samples in time range [start, end] (inclusive)
    ///
    /// # Parameters
    ///
    /// - `start` - Start timestamp (inclusive)
    /// - `end` - End timestamp (inclusive)
    ///
    /// # Returns
    ///
    /// Returns a vector of samples in the specified time range.
    pub fn samples_in_range(&self, start: i64, end: i64) -> Vec<&Sample> {
        self.samples.iter().filter(|s| s.timestamp >= start && s.timestamp <= end).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test Label creation and comparison.
    #[test]
    fn test_label_operations() {
        let label1 = Label::new("job", "api");
        let label2 = Label::new("job", "api");
        let label3 = Label::new("job", "web");

        // Test equality
        assert_eq!(label1, label2);
        assert_ne!(label1, label3);

        // Test fields
        assert_eq!(label1.name, "job");
        assert_eq!(label1.value, "api");

        // Test ordering (for BTreeMap usage)
        assert!(label1 < label3); // "api" < "web"
    }

    /// Test Sample creation and comparison.
    #[test]
    fn test_sample_operations() {
        let sample1 = Sample::new(1000, 42.5);
        let sample2 = Sample::new(1000, 42.5);
        let sample3 = Sample::new(2000, 100.0);

        // Test equality
        assert_eq!(sample1, sample2);
        assert_ne!(sample1, sample3);

        // Test fields
        assert_eq!(sample1.timestamp, 1000);
        assert_eq!(sample1.value, 42.5);
        assert_eq!(sample3.timestamp, 2000);
        assert_eq!(sample3.value, 100.0);
    }

    /// Test TimeSeries creation and sample management.
    #[test]
    fn test_time_series_operations() {
        let labels = vec![Label::new("__name__", "http_requests_total"), Label::new("job", "api")];
        let mut ts = TimeSeries::new(labels.clone());

        // Test initial state
        assert_eq!(ts.labels, labels);
        assert!(ts.samples.is_empty());

        // Add samples in random order
        ts.add_sample(Sample::new(3000, 30.0));
        ts.add_sample(Sample::new(1000, 10.0));
        ts.add_sample(Sample::new(2000, 20.0));

        // Should be sorted by timestamp
        assert_eq!(ts.samples.len(), 3);
        assert_eq!(ts.samples[0].timestamp, 1000);
        assert_eq!(ts.samples[1].timestamp, 2000);
        assert_eq!(ts.samples[2].timestamp, 3000);

        // Replace sample at existing timestamp
        ts.add_sample(Sample::new(2000, 25.0));
        assert_eq!(ts.samples.len(), 3); // Still 3 samples
        assert_eq!(ts.samples[1].value, 25.0); // Value updated
    }

    /// Test samples_in_range filtering.
    #[test]
    fn test_samples_in_range() {
        let mut ts = TimeSeries::new(vec![Label::new("test", "range")]);

        // Add samples at different timestamps
        ts.add_sample(Sample::new(1000, 10.0));
        ts.add_sample(Sample::new(2000, 20.0));
        ts.add_sample(Sample::new(3000, 30.0));
        ts.add_sample(Sample::new(4000, 40.0));
        ts.add_sample(Sample::new(5000, 50.0));

        // Test various ranges
        let range_1500_3500 = ts.samples_in_range(1500, 3500);
        assert_eq!(range_1500_3500.len(), 2);
        assert_eq!(range_1500_3500[0].timestamp, 2000);
        assert_eq!(range_1500_3500[1].timestamp, 3000);

        // Test exact boundaries (inclusive)
        let range_2000_4000 = ts.samples_in_range(2000, 4000);
        assert_eq!(range_2000_4000.len(), 3);
        assert_eq!(range_2000_4000[0].timestamp, 2000);
        assert_eq!(range_2000_4000[2].timestamp, 4000);

        // Test empty range
        let empty_range = ts.samples_in_range(6000, 7000);
        assert!(empty_range.is_empty());

        // Test single point range
        let single_point = ts.samples_in_range(3000, 3000);
        assert_eq!(single_point.len(), 1);
        assert_eq!(single_point[0].timestamp, 3000);
    }

    /// Test edge cases for TimeSeries operations.
    #[test]
    fn test_time_series_edge_cases() {
        let mut ts = TimeSeries::new(vec![]);

        // Empty time series
        assert!(ts.samples.is_empty());
        let empty_range = ts.samples_in_range(0, 1000);
        assert!(empty_range.is_empty());

        // Single sample
        ts.add_sample(Sample::new(500, 5.0));
        assert_eq!(ts.samples.len(), 1);

        // Range tests with single sample
        let before_range = ts.samples_in_range(0, 400);
        assert!(before_range.is_empty());

        let containing_range = ts.samples_in_range(400, 600);
        assert_eq!(containing_range.len(), 1);

        let after_range = ts.samples_in_range(600, 1000);
        assert!(after_range.is_empty());

        // Duplicate timestamps with different values
        ts.add_sample(Sample::new(500, 7.5)); // Should replace
        assert_eq!(ts.samples.len(), 1);
        assert_eq!(ts.samples[0].value, 7.5);
    }
}
