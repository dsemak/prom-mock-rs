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
