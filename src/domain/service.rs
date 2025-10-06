// SPDX-License-Identifier: MIT OR Apache-2.0

//! Configuration service trait definition.
//!
//! This module defines the `ConfigurationService` trait, which is the main interface
//! for interacting with the configuration system. It provides methods for retrieving
//! configuration values, checking for key existence, and managing configuration reloading.

use crate::domain::{ConfigKey, ConfigValue, Result};
use crate::ports::ConfigWatcher;

/// The main configuration service trait.
///
/// This trait defines the primary interface for accessing configuration values.
/// It aggregates multiple configuration sources and provides a unified API for
/// retrieving values with proper precedence handling.
///
/// # Examples
///
/// ```rust
/// use configuration::domain::{ConfigurationService, ConfigKey, ConfigValue, Result};
/// use configuration::ports::ConfigWatcher;
///
/// struct MyConfigService;
///
/// impl ConfigurationService for MyConfigService {
///     fn get(&self, key: &ConfigKey) -> Result<ConfigValue> {
///         // Implementation here
///         Ok(ConfigValue::from("value"))
///     }
///
///     fn get_or_default(&self, key: &ConfigKey, default: &str) -> ConfigValue {
///         self.get(key).unwrap_or_else(|_| ConfigValue::from(default))
///     }
///
///     fn has(&self, key: &ConfigKey) -> bool {
///         self.get(key).is_ok()
///     }
///
///     fn reload(&mut self) -> Result<()> {
///         Ok(())
///     }
///
///     fn register_watcher(&mut self, watcher: Box<dyn ConfigWatcher>) -> Result<()> {
///         Ok(())
///     }
/// }
/// ```
pub trait ConfigurationService {
    /// Retrieves a configuration value for the given key.
    ///
    /// This method queries all configured sources in priority order and returns
    /// the first value found. If no source provides a value for the key, an error
    /// is returned.
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    ///
    /// * `Ok(ConfigValue)` - The configuration value
    /// * `Err(ConfigError)` - The key was not found or an error occurred
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use configuration::domain::{ConfigurationService, ConfigKey, ConfigValue, Result};
    /// # use configuration::ports::ConfigWatcher;
    /// # struct MyConfigService;
    /// # impl ConfigurationService for MyConfigService {
    /// #     fn get(&self, key: &ConfigKey) -> Result<ConfigValue> {
    /// #         Ok(ConfigValue::from("localhost"))
    /// #     }
    /// #     fn get_or_default(&self, key: &ConfigKey, default: &str) -> ConfigValue {
    /// #         self.get(key).unwrap_or_else(|_| ConfigValue::from(default))
    /// #     }
    /// #     fn has(&self, key: &ConfigKey) -> bool { self.get(key).is_ok() }
    /// #     fn reload(&mut self) -> Result<()> { Ok(()) }
    /// #     fn register_watcher(&mut self, watcher: Box<dyn ConfigWatcher>) -> Result<()> { Ok(()) }
    /// # }
    /// let service = MyConfigService;
    /// let key = ConfigKey::from("database.host");
    /// let value = service.get(&key).unwrap();
    /// assert_eq!(value.as_str(), "localhost");
    /// ```
    fn get(&self, key: &ConfigKey) -> Result<ConfigValue>;

    /// Retrieves a configuration value or returns a default value if not found.
    ///
    /// This is a convenience method that returns a default value instead of an error
    /// when the key is not found. This is useful for optional configuration values.
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key to retrieve
    /// * `default` - The default value to return if the key is not found
    ///
    /// # Returns
    ///
    /// The configuration value or the default value
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use configuration::domain::{ConfigurationService, ConfigKey, ConfigValue, Result, ConfigError};
    /// # use configuration::ports::ConfigWatcher;
    /// # struct MyConfigService;
    /// # impl ConfigurationService for MyConfigService {
    /// #     fn get(&self, key: &ConfigKey) -> Result<ConfigValue> {
    /// #         Err(ConfigError::ConfigKeyNotFound {
    /// #             key: key.as_str().to_string(),
    /// #         })
    /// #     }
    /// #     fn get_or_default(&self, key: &ConfigKey, default: &str) -> ConfigValue {
    /// #         self.get(key).unwrap_or_else(|_| ConfigValue::from(default))
    /// #     }
    /// #     fn has(&self, key: &ConfigKey) -> bool { false }
    /// #     fn reload(&mut self) -> Result<()> { Ok(()) }
    /// #     fn register_watcher(&mut self, watcher: Box<dyn ConfigWatcher>) -> Result<()> { Ok(()) }
    /// # }
    /// let service = MyConfigService;
    /// let key = ConfigKey::from("nonexistent.key");
    /// let value = service.get_or_default(&key, "default_value");
    /// assert_eq!(value.as_str(), "default_value");
    /// ```
    fn get_or_default(&self, key: &ConfigKey, default: &str) -> ConfigValue;

    /// Checks if a configuration key exists in any source.
    ///
    /// This method is useful for checking whether a configuration value is available
    /// before attempting to retrieve it.
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key to check
    ///
    /// # Returns
    ///
    /// `true` if the key exists in any source, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use configuration::domain::{ConfigurationService, ConfigKey, ConfigValue, Result};
    /// # use configuration::ports::ConfigWatcher;
    /// # struct MyConfigService;
    /// # impl ConfigurationService for MyConfigService {
    /// #     fn get(&self, key: &ConfigKey) -> Result<ConfigValue> {
    /// #         Ok(ConfigValue::from("value"))
    /// #     }
    /// #     fn get_or_default(&self, key: &ConfigKey, default: &str) -> ConfigValue {
    /// #         self.get(key).unwrap_or_else(|_| ConfigValue::from(default))
    /// #     }
    /// #     fn has(&self, key: &ConfigKey) -> bool { true }
    /// #     fn reload(&mut self) -> Result<()> { Ok(()) }
    /// #     fn register_watcher(&mut self, watcher: Box<dyn ConfigWatcher>) -> Result<()> { Ok(()) }
    /// # }
    /// let service = MyConfigService;
    /// let key = ConfigKey::from("app.name");
    /// assert!(service.has(&key));
    /// ```
    fn has(&self, key: &ConfigKey) -> bool;

    /// Reloads configuration from all sources.
    ///
    /// This method triggers a reload of all configuration sources that support
    /// dynamic reloading. Sources that don't support reloading (like command-line
    /// arguments) are skipped.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All sources were successfully reloaded
    /// * `Err(ConfigError)` - An error occurred while reloading
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use configuration::domain::{ConfigurationService, ConfigKey, ConfigValue, Result};
    /// # use configuration::ports::ConfigWatcher;
    /// # struct MyConfigService;
    /// # impl ConfigurationService for MyConfigService {
    /// #     fn get(&self, key: &ConfigKey) -> Result<ConfigValue> {
    /// #         Ok(ConfigValue::from("value"))
    /// #     }
    /// #     fn get_or_default(&self, key: &ConfigKey, default: &str) -> ConfigValue {
    /// #         self.get(key).unwrap_or_else(|_| ConfigValue::from(default))
    /// #     }
    /// #     fn has(&self, key: &ConfigKey) -> bool { true }
    /// #     fn reload(&mut self) -> Result<()> { Ok(()) }
    /// #     fn register_watcher(&mut self, watcher: Box<dyn ConfigWatcher>) -> Result<()> { Ok(()) }
    /// # }
    /// let mut service = MyConfigService;
    /// service.reload().unwrap();
    /// ```
    fn reload(&mut self) -> Result<()>;

    /// Registers a watcher for configuration changes.
    ///
    /// This method allows clients to be notified when configuration values change.
    /// The watcher will monitor its associated source and trigger callbacks when
    /// changes are detected.
    ///
    /// # Arguments
    ///
    /// * `watcher` - The watcher to register
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The watcher was successfully registered
    /// * `Err(ConfigError)` - An error occurred while registering the watcher
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use configuration::domain::{ConfigurationService, ConfigKey, ConfigValue, Result};
    /// # use configuration::ports::ConfigWatcher;
    /// # use std::sync::Arc;
    /// # struct MyConfigService;
    /// # impl ConfigurationService for MyConfigService {
    /// #     fn get(&self, key: &ConfigKey) -> Result<ConfigValue> {
    /// #         Ok(ConfigValue::from("value"))
    /// #     }
    /// #     fn get_or_default(&self, key: &ConfigKey, default: &str) -> ConfigValue {
    /// #         self.get(key).unwrap_or_else(|_| ConfigValue::from(default))
    /// #     }
    /// #     fn has(&self, key: &ConfigKey) -> bool { true }
    /// #     fn reload(&mut self) -> Result<()> { Ok(()) }
    /// #     fn register_watcher(&mut self, watcher: Box<dyn ConfigWatcher>) -> Result<()> { Ok(()) }
    /// # }
    /// # struct TestWatcher;
    /// # impl ConfigWatcher for TestWatcher {
    /// #     fn watch(&mut self, callback: Arc<dyn Fn(ConfigKey) + Send + Sync>) -> Result<()> { Ok(()) }
    /// #     fn stop(&mut self) -> Result<()> { Ok(()) }
    /// # }
    /// let mut service = MyConfigService;
    /// let watcher = Box::new(TestWatcher);
    /// service.register_watcher(watcher).unwrap();
    /// ```
    fn register_watcher(&mut self, watcher: Box<dyn ConfigWatcher>) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test implementation of ConfigurationService for testing purposes
    struct TestConfigService;

    impl ConfigurationService for TestConfigService {
        fn get(&self, _key: &ConfigKey) -> Result<ConfigValue> {
            Ok(ConfigValue::from("test_value"))
        }

        fn get_or_default(&self, key: &ConfigKey, default: &str) -> ConfigValue {
            self.get(key).unwrap_or_else(|_| ConfigValue::from(default))
        }

        fn has(&self, _key: &ConfigKey) -> bool {
            true
        }

        fn reload(&mut self) -> Result<()> {
            Ok(())
        }

        fn register_watcher(&mut self, _watcher: Box<dyn ConfigWatcher>) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_service_get() {
        let service = TestConfigService;
        let key = ConfigKey::from("test.key");
        let value = service.get(&key).unwrap();
        assert_eq!(value.as_str(), "test_value");
    }

    #[test]
    fn test_service_get_or_default() {
        let service = TestConfigService;
        let key = ConfigKey::from("test.key");
        let value = service.get_or_default(&key, "default");
        assert_eq!(value.as_str(), "test_value");
    }

    #[test]
    fn test_service_has() {
        let service = TestConfigService;
        let key = ConfigKey::from("test.key");
        assert!(service.has(&key));
    }

    #[test]
    fn test_service_reload() {
        let mut service = TestConfigService;
        assert!(service.reload().is_ok());
    }

    #[test]
    fn test_service_register_watcher() {
        use std::sync::Arc;

        struct DummyWatcher;
        impl ConfigWatcher for DummyWatcher {
            fn watch(&mut self, _callback: Arc<dyn Fn(ConfigKey) + Send + Sync>) -> Result<()> {
                Ok(())
            }
            fn stop(&mut self) -> Result<()> {
                Ok(())
            }
        }

        let mut service = TestConfigService;
        let watcher = Box::new(DummyWatcher);
        assert!(service.register_watcher(watcher).is_ok());
    }
}
