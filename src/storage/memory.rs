//! In-memory time series storage implementation.
//!
//! This module provides a basic time series database that stores metrics
//! in memory and supports simple label-based querying with indexing.

use std::sync::{Arc, RwLock};

use fnv::FnvHashMap;

use crate::matchers::LabelMatcher;
use crate::storage::{FullStorage, Label, MetadataStorage, Storage, TimeSeries};

/// In-memory storage for time series data with label indexing.
pub struct MemoryStorage {
    /// Map from series fingerprint to time series
    series: RwLock<FnvHashMap<u64, TimeSeries>>,
    /// Label index for fast lookup
    label_index: RwLock<FnvHashMap<String, FnvHashMap<String, Vec<u64>>>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStorage {
    /// Create a new empty in-memory storage.
    ///
    /// # Returns
    /// Returns a new `MemoryStorage` instance with empty series and label index.
    pub fn new() -> Self {
        Self {
            series: RwLock::new(FnvHashMap::default()),
            label_index: RwLock::new(FnvHashMap::default()),
        }
    }

    /// Generate fingerprint for a label set
    fn fingerprint(labels: &[Label]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for label in labels {
            label.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Update label index for new series
    fn update_label_index(&self, labels: &[Label], fp: u64) {
        let mut index = self.label_index.write().unwrap();

        for label in labels {
            let name_map = index.entry(label.name.clone()).or_default();
            let series_list = name_map.entry(label.value.clone()).or_default();
            if !series_list.contains(&fp) {
                series_list.push(fp);
            }
        }
    }

    /// Check if a time series matches all label matchers
    fn matches_series(ts: &TimeSeries, matchers: &[Arc<dyn LabelMatcher>]) -> bool {
        for matcher in matchers {
            if !matcher.matches(&ts.labels) {
                return false;
            }
        }
        true
    }
}

impl Storage for MemoryStorage {
    fn add_series(&self, ts: TimeSeries) {
        let fp = Self::fingerprint(&ts.labels);

        // Update series store
        {
            let mut series = self.series.write().unwrap();
            if let Some(existing) = series.get_mut(&fp) {
                // Merge samples
                for sample in ts.samples {
                    existing.add_sample(sample);
                }
            } else {
                // Update label index
                self.update_label_index(&ts.labels, fp);
                series.insert(fp, ts);
            }
        }
    }

    fn query_series(&self, matchers: &[Arc<dyn LabelMatcher>]) -> Vec<TimeSeries> {
        let mut results = Vec::new();

        {
            let series = self.series.read().unwrap();

            for ts in series.values() {
                if Self::matches_series(ts, matchers) {
                    results.push(ts.clone());
                }
            }
        }

        results
    }
}

impl MetadataStorage for MemoryStorage {
    fn label_names(&self) -> Vec<String> {
        let index = self.label_index.read().unwrap();
        index.keys().cloned().collect()
    }

    fn label_values(&self, name: &str) -> Vec<String> {
        let index = self.label_index.read().unwrap();

        // Return all unique label values for the given label name, sorted for determinism.
        index
            .get(name)
            .map(|name_map| {
                let mut values: Vec<String> = name_map.keys().cloned().collect();
                values.sort();
                values
            })
            .unwrap_or_default()
    }
}

impl FullStorage for MemoryStorage {}

#[cfg(test)]
mod tests {
    use crate::matchers::{EqualMatcher, NotEqualMatcher};
    use crate::storage::Sample;

    use super::*;

    /// Test basic storage operations: adding series, querying labels and values.
    #[test]
    fn test_memory_storage() {
        let storage = MemoryStorage::new();

        let labels = vec![
            Label::new("__name__", "http_requests_total"),
            Label::new("job", "api"),
            Label::new("method", "GET"),
        ];

        let mut ts = TimeSeries::new(labels);
        ts.add_sample(Sample::new(1000, 42.0));
        ts.add_sample(Sample::new(2000, 43.0));

        storage.add_series(ts);

        let series = storage.query_series(&[]);
        assert_eq!(series.len(), 1);

        let matcher = Arc::new(EqualMatcher::new("job", "api"));
        let series = storage.query_series(&[matcher]);
        assert_eq!(series.len(), 1);

        let names = storage.label_names();
        assert!(names.contains(&"__name__".to_string()));
        assert!(names.contains(&"job".to_string()));

        let values = storage.label_values("job");
        assert!(values.contains(&"api".to_string()));
    }

    /// Test label matcher operations for equality and inequality matching.
    #[test]
    fn test_label_matcher() {
        let storage = MemoryStorage::new();
        let labels = vec![
            Label::new("__name__", "http_requests_total"),
            Label::new("job", "api"),
            Label::new("method", "GET"),
        ];

        let ts = TimeSeries::new(labels.clone());
        storage.add_series(ts);

        let equal_matcher = Arc::new(EqualMatcher::new("job", "api"));
        let results = storage.query_series(&[equal_matcher]);
        assert_eq!(results.len(), 1);

        let not_equal_matcher = Arc::new(NotEqualMatcher::new("job", "web"));
        let results = storage.query_series(&[not_equal_matcher]);
        assert_eq!(results.len(), 1);

        let wrong_matcher = Arc::new(EqualMatcher::new("job", "web"));
        let results = storage.query_series(&[wrong_matcher]);
        assert_eq!(results.len(), 0);
    }
}
