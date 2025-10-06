// SPDX-License-Identifier: MIT OR Apache-2.0

//! Default configuration service implementation.
//!
//! This module provides the default implementation of the `ConfigurationService`
//! trait, which aggregates multiple configuration sources and provides a unified
//! interface for accessing configuration values.

use crate::domain::{ConfigError, ConfigKey, ConfigValue, ConfigurationService, Result};
use crate::ports::{ConfigSource, ConfigWatcher};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Default implementation of the configuration service.
///
/// This service manages multiple configuration sources and queries them in priority
/// order to resolve configuration values. Sources with higher priority values are
/// queried first, and the first value found is returned.
///
/// # Examples
///
/// ```rust
/// use configuration::prelude::*;
/// use configuration::service::DefaultConfigService;
///
/// # fn main() -> Result<()> {
/// // Create a service with environment variables
/// let service = DefaultConfigService::builder()
///     .with_env_vars()
///     .build()?;
///
/// // Or use the default configuration (env + yaml if available)
/// let service = DefaultConfigService::with_defaults("myapp", "com.example")?;
/// # Ok(())
/// # }
/// ```
pub struct DefaultConfigService {
    /// List of configuration sources, maintained in priority order (highest first)
    sources: Vec<Box<dyn ConfigSource>>,
    /// Cache for configuration values
    cache: Arc<RwLock<HashMap<String, ConfigValue>>>,
    /// List of registered watchers
    watchers: Vec<Box<dyn ConfigWatcher>>,
}

impl DefaultConfigService {
    /// Creates a new empty configuration service.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::service::DefaultConfigService;
    ///
    /// let service = DefaultConfigService::new();
    /// ```
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            watchers: Vec::new(),
        }
    }

    /// Creates a new configuration service builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::service::DefaultConfigService;
    ///
    /// # fn main() -> configuration::domain::Result<()> {
    /// let service = DefaultConfigService::builder()
    ///     .with_env_vars()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> ConfigurationServiceBuilder {
        ConfigurationServiceBuilder::new()
    }

    /// Creates a configuration service with default sources.
    ///
    /// This includes environment variables and a YAML file from the default
    /// OS-appropriate location. If the YAML file doesn't exist, only environment
    /// variables will be used.
    ///
    /// # Arguments
    ///
    /// * `app_name` - The application name
    /// * `qualifier` - The organization/qualifier (e.g., "com.example")
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::service::DefaultConfigService;
    ///
    /// # fn main() -> configuration::domain::Result<()> {
    /// let service = DefaultConfigService::with_defaults("myapp", "com.example")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_defaults(app_name: &str, qualifier: &str) -> Result<Self> {
        let mut builder = Self::builder();

        // Always add environment variables
        #[cfg(feature = "env")]
        {
            builder = builder.with_env_vars();
        }

        // Try to add YAML file from default location
        #[cfg(feature = "yaml")]
        {
            use crate::adapters::YamlFileAdapter;
            if let Ok(adapter) = YamlFileAdapter::from_default_location(app_name, qualifier) {
                builder = builder.with_source(Box::new(adapter));
            }
        }

        builder.build()
    }

    /// Adds a configuration source to the service.
    ///
    /// Sources are automatically sorted by priority after being added.
    pub fn add_source(&mut self, source: Box<dyn ConfigSource>) {
        self.sources.push(source);
        self.sort_sources();
        self.invalidate_cache();
    }

    /// Sorts sources by priority (highest first).
    fn sort_sources(&mut self) {
        self.sources.sort_by_key(|b| std::cmp::Reverse(b.priority()));
    }

    /// Invalidates the cache.
    fn invalidate_cache(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Queries all sources for a configuration value, respecting priority order.
    fn query_sources(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> {
        for source in &self.sources {
            match source.get(key) {
                Ok(Some(value)) => return Ok(Some(value)),
                Ok(None) => continue,
                Err(e) => {
                    // Log the error but continue to next source
                    tracing::debug!(
                        "Error querying source '{}' for key '{}': {}",
                        source.name(),
                        key,
                        e
                    );
                    continue;
                }
            }
        }
        Ok(None)
    }
}

impl Default for DefaultConfigService {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurationService for DefaultConfigService {
    fn get(&self, key: &ConfigKey) -> Result<ConfigValue> {
        // Check cache first
        if let Ok(cache) = self.cache.read() {
            if let Some(value) = cache.get(key.as_str()) {
                return Ok(value.clone());
            }
        }

        // Query sources
        let value = self
            .query_sources(key)?
            .ok_or_else(|| ConfigError::ConfigKeyNotFound {
                key: key.as_str().to_string(),
            })?;

        // Update cache
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(key.as_str().to_string(), value.clone());
        }

        Ok(value)
    }

    fn get_or_default(&self, key: &ConfigKey, default: &str) -> ConfigValue {
        self.get(key).unwrap_or_else(|_| ConfigValue::from(default))
    }

    fn has(&self, key: &ConfigKey) -> bool {
        self.get(key).is_ok()
    }

    fn reload(&mut self) -> Result<()> {
        // Reload all sources
        for source in &mut self.sources {
            if let Err(e) = source.reload() {
                tracing::warn!("Failed to reload source '{}': {}", source.name(), e);
            }
        }

        // Invalidate cache after reloading
        self.invalidate_cache();

        Ok(())
    }

    fn register_watcher(&mut self, watcher: Box<dyn ConfigWatcher>) -> Result<()> {
        self.watchers.push(watcher);
        Ok(())
    }
}

/// Builder for constructing a `DefaultConfigService`.
///
/// This builder provides a fluent interface for configuring and creating
/// a configuration service with multiple sources.
///
/// # Examples
///
/// ```rust
/// use configuration::service::ConfigurationServiceBuilder;
///
/// # fn main() -> configuration::domain::Result<()> {
/// let service = ConfigurationServiceBuilder::new()
///     .with_env_vars()
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct ConfigurationServiceBuilder {
    sources: Vec<Box<dyn ConfigSource>>,
}

impl ConfigurationServiceBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Adds a configuration source to the builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::service::ConfigurationServiceBuilder;
    /// use configuration::adapters::EnvVarAdapter;
    ///
    /// # fn main() -> configuration::domain::Result<()> {
    /// let service = ConfigurationServiceBuilder::new()
    ///     .with_source(Box::new(EnvVarAdapter::new()))
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_source(mut self, source: Box<dyn ConfigSource>) -> Self {
        self.sources.push(source);
        self
    }

    /// Adds environment variables as a configuration source.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::service::ConfigurationServiceBuilder;
    ///
    /// # fn main() -> configuration::domain::Result<()> {
    /// let service = ConfigurationServiceBuilder::new()
    ///     .with_env_vars()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "env")]
    pub fn with_env_vars(self) -> Self {
        use crate::adapters::EnvVarAdapter;
        self.with_source(Box::new(EnvVarAdapter::new().lowercase_keys(true)))
    }

    /// Adds environment variables with a prefix as a configuration source.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::service::ConfigurationServiceBuilder;
    ///
    /// # fn main() -> configuration::domain::Result<()> {
    /// let service = ConfigurationServiceBuilder::new()
    ///     .with_env_prefix("MYAPP_")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "env")]
    pub fn with_env_prefix(self, prefix: impl Into<String>) -> Self {
        use crate::adapters::EnvVarAdapter;
        self.with_source(Box::new(
            EnvVarAdapter::with_prefix(prefix).lowercase_keys(true),
        ))
    }

    /// Adds command-line arguments as a configuration source.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::service::ConfigurationServiceBuilder;
    ///
    /// # fn main() -> configuration::domain::Result<()> {
    /// let args = vec!["--key", "value"];
    /// let service = ConfigurationServiceBuilder::new()
    ///     .with_cli_args(args)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "cli")]
    pub fn with_cli_args<S: AsRef<str>>(self, args: Vec<S>) -> Self {
        use crate::adapters::CommandLineAdapter;
        self.with_source(Box::new(CommandLineAdapter::from_args(args)))
    }

    /// Adds a YAML file as a configuration source.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::service::ConfigurationServiceBuilder;
    ///
    /// # fn main() -> configuration::domain::Result<()> {
    /// let service = ConfigurationServiceBuilder::new()
    ///     .with_yaml_file("/etc/myapp/config.yaml")?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "yaml")]
    pub fn with_yaml_file(self, path: impl AsRef<std::path::Path>) -> Result<Self> {
        use crate::adapters::YamlFileAdapter;
        let adapter = YamlFileAdapter::from_file(path)?;
        Ok(self.with_source(Box::new(adapter)))
    }

    /// Builds the configuration service.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::service::ConfigurationServiceBuilder;
    ///
    /// # fn main() -> configuration::domain::Result<()> {
    /// let service = ConfigurationServiceBuilder::new()
    ///     .with_env_vars()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<DefaultConfigService> {
        let mut service = DefaultConfigService::new();

        for source in self.sources {
            service.add_source(source);
        }

        Ok(service)
    }
}

impl Default for ConfigurationServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::ConfigSource;

    // Mock source for testing
    struct MockSource {
        name: String,
        priority: u8,
        values: HashMap<String, String>,
    }

    impl MockSource {
        fn new(name: &str, priority: u8) -> Self {
            Self {
                name: name.to_string(),
                priority,
                values: HashMap::new(),
            }
        }

        fn with_value(mut self, key: &str, value: &str) -> Self {
            self.values.insert(key.to_string(), value.to_string());
            self
        }
    }

    impl ConfigSource for MockSource {
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
            Ok(())
        }
    }

    #[test]
    fn test_default_service_new() {
        let service = DefaultConfigService::new();
        assert_eq!(service.sources.len(), 0);
    }

    #[test]
    fn test_default_service_add_source() {
        let mut service = DefaultConfigService::new();
        let source = Box::new(MockSource::new("test", 1));

        service.add_source(source);
        assert_eq!(service.sources.len(), 1);
    }

    #[test]
    fn test_default_service_priority_order() {
        let mut service = DefaultConfigService::new();

        // Add sources in reverse priority order
        service.add_source(Box::new(MockSource::new("low", 1)));
        service.add_source(Box::new(MockSource::new("high", 3)));
        service.add_source(Box::new(MockSource::new("medium", 2)));

        // Verify they're sorted by priority (highest first)
        assert_eq!(service.sources[0].name(), "high");
        assert_eq!(service.sources[1].name(), "medium");
        assert_eq!(service.sources[2].name(), "low");
    }

    #[test]
    fn test_default_service_get_from_single_source() {
        let mut service = DefaultConfigService::new();
        let source = MockSource::new("test", 1).with_value("key", "value");

        service.add_source(Box::new(source));

        let key = ConfigKey::from("key");
        let value = service.get(&key).unwrap();
        assert_eq!(value.as_str(), "value");
    }

    #[test]
    fn test_default_service_get_precedence() {
        let mut service = DefaultConfigService::new();

        // Add sources with different priorities and values for the same key
        service.add_source(Box::new(
            MockSource::new("low", 1).with_value("key", "low_value"),
        ));
        service.add_source(Box::new(
            MockSource::new("high", 3).with_value("key", "high_value"),
        ));
        service.add_source(Box::new(
            MockSource::new("medium", 2).with_value("key", "medium_value"),
        ));

        let key = ConfigKey::from("key");
        let value = service.get(&key).unwrap();

        // Should get value from highest priority source
        assert_eq!(value.as_str(), "high_value");
    }

    #[test]
    fn test_default_service_get_missing_key() {
        let mut service = DefaultConfigService::new();
        service.add_source(Box::new(
            MockSource::new("test", 1).with_value("key", "value"),
        ));

        let key = ConfigKey::from("nonexistent");
        let result = service.get(&key);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::ConfigKeyNotFound { .. }
        ));
    }

    #[test]
    fn test_default_service_get_or_default() {
        let mut service = DefaultConfigService::new();
        service.add_source(Box::new(
            MockSource::new("test", 1).with_value("key", "value"),
        ));

        let key = ConfigKey::from("nonexistent");
        let value = service.get_or_default(&key, "default_value");

        assert_eq!(value.as_str(), "default_value");
    }

    #[test]
    fn test_default_service_has() {
        let mut service = DefaultConfigService::new();
        service.add_source(Box::new(
            MockSource::new("test", 1).with_value("key", "value"),
        ));

        assert!(service.has(&ConfigKey::from("key")));
        assert!(!service.has(&ConfigKey::from("nonexistent")));
    }

    #[test]
    fn test_default_service_cache() {
        let mut service = DefaultConfigService::new();
        service.add_source(Box::new(
            MockSource::new("test", 1).with_value("key", "value"),
        ));

        let key = ConfigKey::from("key");

        // First call should populate cache
        let value1 = service.get(&key).unwrap();

        // Second call should use cache
        let value2 = service.get(&key).unwrap();

        assert_eq!(value1.as_str(), value2.as_str());
    }

    #[test]
    fn test_default_service_reload() {
        let mut service = DefaultConfigService::new();
        service.add_source(Box::new(
            MockSource::new("test", 1).with_value("key", "value"),
        ));

        assert!(service.reload().is_ok());
    }

    #[test]
    fn test_builder_new() {
        let builder = ConfigurationServiceBuilder::new();
        assert_eq!(builder.sources.len(), 0);
    }

    #[test]
    fn test_builder_with_source() {
        let source = Box::new(MockSource::new("test", 1));
        let builder = ConfigurationServiceBuilder::new().with_source(source);

        assert_eq!(builder.sources.len(), 1);
    }

    #[test]
    fn test_builder_build() {
        let service = ConfigurationServiceBuilder::new()
            .with_source(Box::new(MockSource::new("test", 1)))
            .build()
            .unwrap();

        assert_eq!(service.sources.len(), 1);
    }

    #[test]
    #[cfg(feature = "env")]
    fn test_builder_with_env_vars() {
        let service = ConfigurationServiceBuilder::new()
            .with_env_vars()
            .build()
            .unwrap();

        assert_eq!(service.sources.len(), 1);
        assert_eq!(service.sources[0].name(), "env");
    }

    #[test]
    fn test_builder_default() {
        let builder = ConfigurationServiceBuilder::default();
        assert_eq!(builder.sources.len(), 0);
    }

    #[test]
    fn test_service_default() {
        let service = DefaultConfigService::default();
        assert_eq!(service.sources.len(), 0);
    }
}
