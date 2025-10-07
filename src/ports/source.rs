// SPDX-License-Identifier: MIT OR Apache-2.0

//! Configuration source trait definition.
//!
//! This module defines the `ConfigSource` trait, which is the primary port (interface)
//! for implementing different configuration sources. Any configuration source (environment
//! variables, files, remote services, etc.) must implement this trait.

use crate::domain::{ConfigKey, ConfigValue, Result};

/// A trait for configuration sources.
///
/// This trait defines the interface that all configuration sources must implement.
/// It provides methods for retrieving configuration values, listing all available keys,
/// and reloading the source if supported.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow for use in multi-threaded contexts.
///
/// # Priority
///
/// Each source has a priority value (0-255) that determines the order in which sources
/// are queried. Higher priority values take precedence over lower ones. The typical
/// priority values are:
///
/// - **3 (highest)**: Command-line arguments
/// - **2**: Environment variables
/// - **1 (lowest)**: Configuration files and remote services
///
/// # Examples
///
/// ```rust
/// use hexcfg::ports::ConfigSource;
/// use hexcfg::domain::{ConfigKey, ConfigValue, Result};
///
/// struct MySource;
///
/// impl ConfigSource for MySource {
///     fn name(&self) -> &str {
///         "my-source"
///     }
///
///     fn priority(&self) -> u8 {
///         1
///     }
///
///     fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> {
///         // Implementation here
///         Ok(None)
///     }
///
///     fn all_keys(&self) -> Result<Vec<ConfigKey>> {
///         Ok(vec![])
///     }
///
///     fn reload(&mut self) -> Result<()> {
///         Ok(())
///     }
/// }
/// ```
pub trait ConfigSource: Send + Sync {
    /// Returns the name of this configuration source.
    ///
    /// This name is used for logging, error messages, and debugging. It should be
    /// a short, descriptive identifier like "env", "yaml-file", "etcd", etc.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hexcfg::ports::ConfigSource;
    /// # use hexcfg::domain::{ConfigKey, ConfigValue, Result};
    /// # struct MySource;
    /// # impl ConfigSource for MySource {
    /// #     fn name(&self) -> &str { "my-source" }
    /// #     fn priority(&self) -> u8 { 1 }
    /// #     fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> { Ok(None) }
    /// #     fn all_keys(&self) -> Result<Vec<ConfigKey>> { Ok(vec![]) }
    /// #     fn reload(&mut self) -> Result<()> { Ok(()) }
    /// # }
    /// let source = MySource;
    /// assert_eq!(source.name(), "my-source");
    /// ```
    fn name(&self) -> &str;

    /// Returns the priority of this configuration source.
    ///
    /// Priority determines the order in which sources are queried. Higher values
    /// take precedence over lower values. When multiple sources provide a value
    /// for the same key, the value from the source with the highest priority is used.
    ///
    /// # Priority Guidelines
    ///
    /// - **3**: Command-line arguments (highest priority)
    /// - **2**: Environment variables
    /// - **1**: Files and remote services (lowest priority)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hexcfg::ports::ConfigSource;
    /// # use hexcfg::domain::{ConfigKey, ConfigValue, Result};
    /// # struct MySource;
    /// # impl ConfigSource for MySource {
    /// #     fn name(&self) -> &str { "my-source" }
    /// #     fn priority(&self) -> u8 { 2 }
    /// #     fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> { Ok(None) }
    /// #     fn all_keys(&self) -> Result<Vec<ConfigKey>> { Ok(vec![]) }
    /// #     fn reload(&mut self) -> Result<()> { Ok(()) }
    /// # }
    /// let source = MySource;
    /// assert_eq!(source.priority(), 2);
    /// ```
    fn priority(&self) -> u8;

    /// Retrieves a configuration value for the given key.
    ///
    /// Returns `Ok(Some(value))` if the key exists in this source, `Ok(None)` if the
    /// key does not exist, or `Err` if an error occurred while retrieving the value.
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    ///
    /// * `Ok(Some(ConfigValue))` - The value was found
    /// * `Ok(None)` - The key does not exist in this source
    /// * `Err(ConfigError)` - An error occurred
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hexcfg::ports::ConfigSource;
    /// # use hexcfg::domain::{ConfigKey, ConfigValue, Result};
    /// # struct MySource;
    /// # impl ConfigSource for MySource {
    /// #     fn name(&self) -> &str { "my-source" }
    /// #     fn priority(&self) -> u8 { 1 }
    /// #     fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> {
    /// #         if key.as_str() == "app.name" {
    /// #             Ok(Some(ConfigValue::from("MyApp")))
    /// #         } else {
    /// #             Ok(None)
    /// #         }
    /// #     }
    /// #     fn all_keys(&self) -> Result<Vec<ConfigKey>> { Ok(vec![]) }
    /// #     fn reload(&mut self) -> Result<()> { Ok(()) }
    /// # }
    /// let source = MySource;
    /// let key = ConfigKey::from("app.name");
    /// let value = source.get(&key).unwrap();
    /// assert!(value.is_some());
    /// ```
    fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>>;

    /// Returns all configuration keys available in this source.
    ///
    /// This method is useful for discovering available configuration options,
    /// debugging, and validation.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ConfigKey>)` - A list of all available keys
    /// * `Err(ConfigError)` - An error occurred while retrieving keys
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hexcfg::ports::ConfigSource;
    /// # use hexcfg::domain::{ConfigKey, ConfigValue, Result};
    /// # struct MySource;
    /// # impl ConfigSource for MySource {
    /// #     fn name(&self) -> &str { "my-source" }
    /// #     fn priority(&self) -> u8 { 1 }
    /// #     fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> { Ok(None) }
    /// #     fn all_keys(&self) -> Result<Vec<ConfigKey>> {
    /// #         Ok(vec![ConfigKey::from("key1"), ConfigKey::from("key2")])
    /// #     }
    /// #     fn reload(&mut self) -> Result<()> { Ok(()) }
    /// # }
    /// let source = MySource;
    /// let keys = source.all_keys().unwrap();
    /// assert_eq!(keys.len(), 2);
    /// ```
    fn all_keys(&self) -> Result<Vec<ConfigKey>>;

    /// Reloads the configuration from the source.
    ///
    /// This method allows sources to refresh their data from the underlying storage.
    /// For sources that don't support reloading (like command-line arguments), this
    /// can be a no-op that returns `Ok(())`.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The source was successfully reloaded (or reloading is not applicable)
    /// * `Err(ConfigError)` - An error occurred while reloading
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hexcfg::ports::ConfigSource;
    /// # use hexcfg::domain::{ConfigKey, ConfigValue, Result};
    /// # struct MySource;
    /// # impl ConfigSource for MySource {
    /// #     fn name(&self) -> &str { "my-source" }
    /// #     fn priority(&self) -> u8 { 1 }
    /// #     fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> { Ok(None) }
    /// #     fn all_keys(&self) -> Result<Vec<ConfigKey>> { Ok(vec![]) }
    /// #     fn reload(&mut self) -> Result<()> {
    /// #         // Reload logic here
    /// #         Ok(())
    /// #     }
    /// # }
    /// let mut source = MySource;
    /// source.reload().unwrap();
    /// ```
    fn reload(&mut self) -> Result<()>;

    /// Retrieves a configuration value for the given key string.
    ///
    /// This is a convenience method that automatically converts a string slice
    /// into a `ConfigKey`. It's equivalent to calling `get(&ConfigKey::from(key))`.
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key as a string slice
    ///
    /// # Returns
    ///
    /// * `Ok(Some(ConfigValue))` - The value was found
    /// * `Ok(None)` - The key does not exist in this source
    /// * `Err(ConfigError)` - An error occurred
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hexcfg::ports::ConfigSource;
    /// # use hexcfg::domain::{ConfigKey, ConfigValue, Result};
    /// # struct MySource;
    /// # impl ConfigSource for MySource {
    /// #     fn name(&self) -> &str { "my-source" }
    /// #     fn priority(&self) -> u8 { 1 }
    /// #     fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> {
    /// #         if key.as_str() == "app.name" {
    /// #             Ok(Some(ConfigValue::from("MyApp")))
    /// #         } else {
    /// #             Ok(None)
    /// #         }
    /// #     }
    /// #     fn all_keys(&self) -> Result<Vec<ConfigKey>> { Ok(vec![]) }
    /// #     fn reload(&mut self) -> Result<()> { Ok(()) }
    /// # }
    /// let source = MySource;
    /// // No need to create a ConfigKey manually!
    /// let value = source.get_str("app.name").unwrap();
    /// assert!(value.is_some());
    /// ```
    fn get_str(&self, key: &str) -> Result<Option<ConfigValue>> {
        self.get(&ConfigKey::from(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test implementation of ConfigSource for testing purposes
    struct TestSource {
        name: String,
        priority: u8,
    }

    impl ConfigSource for TestSource {
        fn name(&self) -> &str {
            &self.name
        }

        fn priority(&self) -> u8 {
            self.priority
        }

        fn get(&self, _key: &ConfigKey) -> Result<Option<ConfigValue>> {
            Ok(None)
        }

        fn all_keys(&self) -> Result<Vec<ConfigKey>> {
            Ok(vec![])
        }

        fn reload(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_config_source_name() {
        let source = TestSource {
            name: "test-source".to_string(),
            priority: 1,
        };
        assert_eq!(source.name(), "test-source");
    }

    #[test]
    fn test_config_source_priority() {
        let source = TestSource {
            name: "test-source".to_string(),
            priority: 2,
        };
        assert_eq!(source.priority(), 2);
    }

    #[test]
    fn test_config_source_get_returns_none() {
        let source = TestSource {
            name: "test-source".to_string(),
            priority: 1,
        };
        let key = ConfigKey::from("nonexistent");
        let result = source.get(&key).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_config_source_all_keys_empty() {
        let source = TestSource {
            name: "test-source".to_string(),
            priority: 1,
        };
        let keys = source.all_keys().unwrap();
        assert_eq!(keys.len(), 0);
    }

    #[test]
    fn test_config_source_reload() {
        let mut source = TestSource {
            name: "test-source".to_string(),
            priority: 1,
        };
        assert!(source.reload().is_ok());
    }

    #[test]
    fn test_config_source_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn ConfigSource>>();
    }
}
