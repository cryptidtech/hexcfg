// SPDX-License-Identifier: MIT OR Apache-2.0

//! Environment variable configuration source adapter.
//!
//! This module provides an adapter that reads configuration values from
//! environment variables.

use crate::domain::{ConfigKey, ConfigValue, Result};
use crate::ports::ConfigSource;
use std::collections::HashMap;
use std::env;
use std::sync::RwLock;

/// Maximum length for environment variable keys (prevents DoS)
const MAX_ENV_KEY_LEN: usize = 512;

/// Maximum length for environment variable values (prevents DoS)
const MAX_ENV_VALUE_LEN: usize = 1048576; // 1MB

/// Configuration source adapter for environment variables.
///
/// This adapter reads configuration values from environment variables. It supports
/// optional prefix filtering (e.g., only read variables starting with "APP_") and
/// key transformation (e.g., converting underscores to dots).
///
/// # Priority
///
/// Environment variables have a priority of 2, which means they override configuration
/// files (priority 1) but are overridden by command-line arguments (priority 3).
///
/// # Examples
///
/// ```rust
/// use configuration::adapters::EnvVarAdapter;
/// use configuration::ports::ConfigSource;
///
/// // Read all environment variables
/// let adapter = EnvVarAdapter::new();
///
/// // Read only variables with a specific prefix
/// let adapter = EnvVarAdapter::with_prefix("APP_");
/// ```
#[derive(Debug)]
pub struct EnvVarAdapter {
    /// Optional prefix to filter environment variables
    prefix: Option<String>,
    /// Whether to convert keys to lowercase
    lowercase_keys: bool,
    /// Whether to replace underscores with dots
    replace_underscores: bool,
    /// Cached environment variables with interior mutability for thread-safe lazy loading
    cache: RwLock<Option<HashMap<String, String>>>,
}

impl EnvVarAdapter {
    /// Creates a new environment variable adapter without prefix filtering.
    ///
    /// This will read all environment variables available to the process.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::adapters::EnvVarAdapter;
    ///
    /// let adapter = EnvVarAdapter::new();
    /// ```
    pub fn new() -> Self {
        Self {
            prefix: None,
            lowercase_keys: false,
            replace_underscores: true,
            cache: RwLock::new(None),
        }
    }

    /// Creates a new environment variable adapter with prefix filtering.
    ///
    /// Only environment variables starting with the given prefix will be read.
    /// The prefix is stripped from the key when storing values.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix to filter environment variables (e.g., "APP_")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::adapters::EnvVarAdapter;
    ///
    /// let adapter = EnvVarAdapter::with_prefix("MYAPP_");
    /// ```
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: Some(prefix.into()),
            lowercase_keys: false,
            replace_underscores: true,
            cache: RwLock::new(None),
        }
    }

    /// Sets whether to convert keys to lowercase.
    ///
    /// When enabled, environment variable names are converted to lowercase
    /// before being used as keys.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::adapters::EnvVarAdapter;
    ///
    /// let adapter = EnvVarAdapter::new()
    ///     .lowercase_keys(true);
    /// ```
    pub fn lowercase_keys(mut self, enabled: bool) -> Self {
        self.lowercase_keys = enabled;
        self
    }

    /// Sets whether to replace underscores with dots in keys.
    ///
    /// When enabled (default), underscores in environment variable names are
    /// replaced with dots to match the standard configuration key format.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::adapters::EnvVarAdapter;
    ///
    /// let adapter = EnvVarAdapter::new()
    ///     .replace_underscores(false);
    /// ```
    pub fn replace_underscores(mut self, enabled: bool) -> Self {
        self.replace_underscores = enabled;
        self
    }

    /// Creates an adapter with pre-populated values for testing.
    ///
    /// **Note**: This method is primarily intended for testing and should not
    /// be used in production code. Use `new()` or `with_prefix()` for normal usage.
    ///
    /// This method is useful for testing scenarios where you want to
    /// provide specific environment variable values without actually
    /// setting them in the process environment.
    ///
    /// # Arguments
    ///
    /// * `values` - A HashMap of key-value pairs to use as environment variables
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::adapters::EnvVarAdapter;
    /// use std::collections::HashMap;
    ///
    /// let mut values = HashMap::new();
    /// values.insert("test.key".to_string(), "test_value".to_string());
    ///
    /// let adapter = EnvVarAdapter::with_values(values);
    /// ```
    pub fn with_values(values: HashMap<String, String>) -> Self {
        Self {
            prefix: None,
            lowercase_keys: false,
            replace_underscores: false,
            cache: RwLock::new(Some(values)),
        }
    }

    /// Loads environment variables into a new HashMap.
    fn load(&self) -> HashMap<String, String> {
        let mut cache = HashMap::new();

        for (key, value) in env::vars() {
            // Validate input sizes to prevent DoS
            if key.len() > MAX_ENV_KEY_LEN || value.len() > MAX_ENV_VALUE_LEN {
                tracing::debug!(
                    "Skipping oversized environment variable: key_len={}, value_len={} (max key={}, max value={})",
                    key.len(),
                    value.len(),
                    MAX_ENV_KEY_LEN,
                    MAX_ENV_VALUE_LEN
                );
                continue;
            }

            // Apply prefix filtering
            let key = if let Some(prefix) = &self.prefix {
                if !key.starts_with(prefix) {
                    continue;
                }
                // Strip prefix - this is safe because we just checked starts_with
                match key.strip_prefix(prefix) {
                    Some(stripped) => stripped.to_string(),
                    None => continue, // Should never happen, but skip if it does
                }
            } else {
                key
            };

            // Apply transformations
            let mut transformed_key = key;
            if self.lowercase_keys {
                transformed_key = transformed_key.to_lowercase();
            }
            if self.replace_underscores {
                transformed_key = transformed_key.replace('_', ".");
            }

            cache.insert(transformed_key, value);
        }

        tracing::debug!(
            "Loaded {} environment variables (prefix={:?}, lowercase={}, replace_underscores={})",
            cache.len(),
            self.prefix,
            self.lowercase_keys,
            self.replace_underscores
        );

        cache
    }

    /// Gets the cache, loading it if necessary. Uses interior mutability for thread-safe lazy loading.
    fn get_cache(&self) -> HashMap<String, String> {
        // Try to read from cache first
        {
            let cache_guard = self.cache.read().unwrap();
            if let Some(cache) = cache_guard.as_ref() {
                return cache.clone();
            }
        }

        // Cache miss - need to load
        let new_cache = self.load();

        // Write to cache
        {
            let mut cache_guard = self.cache.write().unwrap();
            *cache_guard = Some(new_cache.clone());
        }

        new_cache
    }
}

impl Default for EnvVarAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for EnvVarAdapter {
    fn name(&self) -> &str {
        "env"
    }

    fn priority(&self) -> u8 {
        2
    }

    fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> {
        let cache = self.get_cache();

        Ok(cache
            .get(key.as_str())
            .map(|v| ConfigValue::from(v.as_str())))
    }

    fn all_keys(&self) -> Result<Vec<ConfigKey>> {
        let cache = self.get_cache();

        Ok(cache.keys().map(|k| ConfigKey::from(k.as_str())).collect())
    }

    fn reload(&mut self) -> Result<()> {
        // Clear cache to force reload on next access
        let mut cache_guard = self.cache.write().unwrap();
        *cache_guard = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Helper to set and clean up environment variables
    struct EnvGuard {
        keys: Vec<String>,
    }

    impl EnvGuard {
        fn new() -> Self {
            EnvGuard { keys: Vec::new() }
        }

        fn set(&mut self, key: &str, value: &str) {
            env::set_var(key, value);
            self.keys.push(key.to_string());
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for key in &self.keys {
                env::remove_var(key);
            }
        }
    }

    #[test]
    fn test_env_adapter_name() {
        let adapter = EnvVarAdapter::new();
        assert_eq!(adapter.name(), "env");
    }

    #[test]
    fn test_env_adapter_priority() {
        let adapter = EnvVarAdapter::new();
        assert_eq!(adapter.priority(), 2);
    }

    #[test]
    fn test_env_adapter_get() {
        let mut guard = EnvGuard::new();
        guard.set("TEST_CONFIG_VAR", "test_value");

        let adapter = EnvVarAdapter::new();
        let key = ConfigKey::from("TEST.CONFIG.VAR");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "test_value");
    }

    #[test]
    fn test_env_adapter_get_nonexistent() {
        let adapter = EnvVarAdapter::new();
        let key = ConfigKey::from("NONEXISTENT_VAR_12345");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_none());
    }

    #[test]
    fn test_env_adapter_with_prefix() {
        let mut guard = EnvGuard::new();
        guard.set("MYAPP_DATABASE_HOST", "localhost");
        guard.set("OTHER_VAR", "should_not_appear");

        let adapter = EnvVarAdapter::with_prefix("MYAPP_");
        let key = ConfigKey::from("DATABASE.HOST");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "localhost");

        // OTHER_VAR should not be accessible
        let key = ConfigKey::from("OTHER.VAR");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_none());
    }

    #[test]
    fn test_env_adapter_lowercase_keys() {
        let mut guard = EnvGuard::new();
        guard.set("UPPER_CASE_KEY", "value");

        let adapter = EnvVarAdapter::new().lowercase_keys(true);
        let key = ConfigKey::from("upper.case.key");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "value");
    }

    #[test]
    fn test_env_adapter_no_replace_underscores() {
        let mut guard = EnvGuard::new();
        guard.set("MY_VAR", "value");

        let adapter = EnvVarAdapter::new().replace_underscores(false);
        let key = ConfigKey::from("MY_VAR");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "value");
    }

    #[test]
    fn test_env_adapter_all_keys() {
        let mut guard = EnvGuard::new();
        guard.set("TEST_KEY_1", "value1");
        guard.set("TEST_KEY_2", "value2");

        let adapter = EnvVarAdapter::with_prefix("TEST_");
        let keys = adapter.all_keys().unwrap();

        assert!(keys.len() >= 2);
        assert!(keys.contains(&ConfigKey::from("KEY.1")));
        assert!(keys.contains(&ConfigKey::from("KEY.2")));
    }

    #[test]
    fn test_env_adapter_reload() {
        let mut guard = EnvGuard::new();
        guard.set("RELOAD_TEST", "initial");

        let mut adapter = EnvVarAdapter::with_prefix("RELOAD_");

        // First load
        let key = ConfigKey::from("TEST");
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "initial");

        // Change environment variable
        guard.set("RELOAD_TEST", "updated");

        // Reload
        adapter.reload().unwrap();

        // Value should be updated
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "updated");
    }

    #[test]
    fn test_env_adapter_default() {
        let adapter = EnvVarAdapter::default();
        assert_eq!(adapter.name(), "env");
        assert_eq!(adapter.priority(), 2);
    }

    #[test]
    fn test_env_adapter_combined_transformations() {
        let mut guard = EnvGuard::new();
        guard.set("APP_DATABASE_HOST", "localhost");

        let adapter = EnvVarAdapter::with_prefix("APP_")
            .lowercase_keys(true)
            .replace_underscores(true);

        let key = ConfigKey::from("database.host");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "localhost");
    }
}
