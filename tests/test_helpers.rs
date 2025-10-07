// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test utilities and mock implementations for testing.
//!
//! This module provides helper functions and mock implementations
//! that can be used across different test files.

use hexcfg::domain::{ConfigError, ConfigKey, ConfigValue, Result};
use hexcfg::ports::ConfigSource;
use std::collections::HashMap;

/// A mock configuration source for testing.
///
/// This allows tests to easily create a source with predefined values
/// and custom priority.
#[derive(Debug, Clone)]
pub struct MockConfigSource {
    name: String,
    priority: u8,
    values: HashMap<String, String>,
    should_fail_reload: bool,
}

impl MockConfigSource {
    /// Creates a new mock source with the given name and priority.
    pub fn new(name: impl Into<String>, priority: u8) -> Self {
        Self {
            name: name.into(),
            priority,
            values: HashMap::new(),
            should_fail_reload: false,
        }
    }

    /// Adds a value to the mock source.
    pub fn with_value(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }

    /// Adds multiple values to the mock source.
    pub fn with_values(mut self, values: HashMap<String, String>) -> Self {
        self.values.extend(values);
        self
    }

    /// Sets whether reload should fail.
    pub fn with_failing_reload(mut self, should_fail: bool) -> Self {
        self.should_fail_reload = should_fail;
        self
    }

    /// Updates a value in the mock source.
    pub fn update_value(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    /// Removes a value from the mock source.
    pub fn remove_value(&mut self, key: &str) {
        self.values.remove(key);
    }
}

impl ConfigSource for MockConfigSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> {
        Ok(self
            .values
            .get(key.as_str())
            .map(|v| ConfigValue::from(v.as_str())))
    }

    fn all_keys(&self) -> Result<Vec<ConfigKey>> {
        Ok(self
            .values
            .keys()
            .map(|k| ConfigKey::from(k.as_str()))
            .collect())
    }

    fn reload(&mut self) -> Result<()> {
        if self.should_fail_reload {
            Err(ConfigError::SourceError {
                source_name: self.name.clone(),
                message: "Mock reload failure".to_string(),
                source: None,
            })
        } else {
            Ok(())
        }
    }
}

/// Creates a temporary YAML file with the given content.
///
/// Returns a NamedTempFile that will be automatically deleted when dropped.
#[cfg(test)]
pub fn create_temp_yaml(content: &str) -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().unwrap();
    write!(file, "{}", content).unwrap();
    file.flush().unwrap();
    file
}

/// Creates a mock configuration source with common test values.
pub fn create_test_source() -> MockConfigSource {
    MockConfigSource::new("test", 1)
        .with_value("string.value", "test")
        .with_value("int.value", "42")
        .with_value("bool.value", "true")
        .with_value("float.value", "3.14")
}

/// Creates multiple mock sources with different priorities for precedence testing.
pub fn create_precedence_sources() -> (MockConfigSource, MockConfigSource, MockConfigSource) {
    let low = MockConfigSource::new("low_priority", 1)
        .with_value("key1", "from_low")
        .with_value("key2", "low_value");

    let medium = MockConfigSource::new("medium_priority", 2)
        .with_value("key1", "from_medium")
        .with_value("key3", "medium_value");

    let high = MockConfigSource::new("high_priority", 3)
        .with_value("key1", "from_high")
        .with_value("key4", "high_value");

    (low, medium, high)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexcfg::domain::ConfigKey;

    #[test]
    fn test_mock_source_basic() {
        let source = MockConfigSource::new("test", 1).with_value("key", "value");

        assert_eq!(source.name(), "test");
        assert_eq!(source.priority(), 1);

        let value = source.get(&ConfigKey::from("key")).unwrap().unwrap();
        assert_eq!(value.as_str(), "value");
    }

    #[test]
    fn test_mock_source_all_keys() {
        let source = MockConfigSource::new("test", 1)
            .with_value("key1", "value1")
            .with_value("key2", "value2");

        let keys = source.all_keys().unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&ConfigKey::from("key1")));
        assert!(keys.contains(&ConfigKey::from("key2")));
    }

    #[test]
    fn test_mock_source_update() {
        let mut source = MockConfigSource::new("test", 1).with_value("key", "original");

        source.update_value("key", "updated");

        let value = source.get(&ConfigKey::from("key")).unwrap().unwrap();
        assert_eq!(value.as_str(), "updated");
    }

    #[test]
    fn test_mock_source_reload_failure() {
        let mut source = MockConfigSource::new("test", 1).with_failing_reload(true);

        let result = source.reload();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_test_source() {
        let source = create_test_source();
        assert_eq!(source.name(), "test");

        let value = source
            .get(&ConfigKey::from("string.value"))
            .unwrap()
            .unwrap();
        assert_eq!(value.as_str(), "test");
    }

    #[test]
    fn test_precedence_sources() {
        let (low, medium, high) = create_precedence_sources();

        assert_eq!(low.priority(), 1);
        assert_eq!(medium.priority(), 2);
        assert_eq!(high.priority(), 3);

        // Verify they all have "key1" but with different values
        let low_val = low.get(&ConfigKey::from("key1")).unwrap().unwrap();
        let med_val = medium.get(&ConfigKey::from("key1")).unwrap().unwrap();
        let high_val = high.get(&ConfigKey::from("key1")).unwrap().unwrap();

        assert_eq!(low_val.as_str(), "from_low");
        assert_eq!(med_val.as_str(), "from_medium");
        assert_eq!(high_val.as_str(), "from_high");
    }
}
