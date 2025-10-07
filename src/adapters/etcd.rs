// SPDX-License-Identifier: MIT OR Apache-2.0

//! etcd configuration source adapter.
//!
//! This module provides an adapter that reads configuration values from etcd.

use crate::domain::{ConfigError, ConfigKey, ConfigValue, Result};
use crate::ports::ConfigSource;
use etcd_client::{Client, GetOptions};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Shared runtime for reload operations to avoid expensive runtime creation on every reload
static RELOAD_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Runtime::new().expect("Failed to create reload runtime for etcd adapter")
});

/// Configuration source adapter for etcd.
///
/// This adapter reads configuration values from an etcd cluster. It supports
/// key prefix filtering for namespacing and includes connection retry logic.
///
/// # Priority
///
/// etcd has a default priority of 1, but this can be customized.
///
/// # Examples
///
/// ```rust,no_run
/// use configuration::adapters::EtcdAdapter;
/// use configuration::ports::ConfigSource;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Connect to etcd with prefix
/// let adapter = EtcdAdapter::new(vec!["localhost:2379"], Some("myapp/"))
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct EtcdAdapter {
    /// etcd client
    client: Arc<Client>,
    /// etcd endpoints for reconnection
    endpoints: Vec<String>,
    /// Key prefix for namespacing
    prefix: Option<String>,
    /// Priority for this source
    priority: u8,
    /// Cached configuration values
    cache: HashMap<String, String>,
}

impl fmt::Debug for EtcdAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EtcdAdapter")
            .field("client", &"<etcd::Client>")
            .field("endpoints", &self.endpoints)
            .field("prefix", &self.prefix)
            .field("priority", &self.priority)
            .field("cache", &self.cache)
            .finish()
    }
}

impl EtcdAdapter {
    /// Validates prefix to prevent injection attacks
    fn validate_prefix(prefix: &str) -> Result<()> {
        // Disallow characters that could cause issues in etcd keys
        if prefix.contains(['\0', '\n', '\r']) {
            return Err(ConfigError::SourceError {
                source_name: "etcd".to_string(),
                message: "Prefix contains invalid characters".to_string(),
                source: None,
            });
        }
        Ok(())
    }

    /// Creates a new etcd adapter with the given endpoints.
    ///
    /// # Arguments
    ///
    /// * `endpoints` - List of etcd endpoints (e.g., `["localhost:2379"]`)
    /// * `prefix` - Optional key prefix for namespacing (e.g., `"myapp/"`)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::adapters::EtcdAdapter;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let adapter = EtcdAdapter::new(vec!["localhost:2379"], Some("myapp/"))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new<S: AsRef<str>>(endpoints: Vec<S>, prefix: Option<&str>) -> Result<Self> {
        // Validate prefix if provided
        if let Some(p) = prefix {
            Self::validate_prefix(p)?;
        }

        let endpoints: Vec<String> = endpoints.iter().map(|s| s.as_ref().to_string()).collect();

        let client =
            Client::connect(&endpoints, None)
                .await
                .map_err(|e| ConfigError::SourceError {
                    source_name: "etcd".to_string(),
                    message: format!("Failed to connect to etcd: {}", e),
                    source: Some(Box::new(e)),
                })?;

        let mut adapter = Self {
            client: Arc::new(client),
            endpoints: endpoints.clone(),
            prefix: prefix.map(|s| s.to_string()),
            priority: 1,
            cache: HashMap::new(),
        };

        // Initial load of all keys
        adapter.load_all_keys().await?;

        Ok(adapter)
    }

    /// Creates a new etcd adapter with a custom priority.
    ///
    /// # Arguments
    ///
    /// * `endpoints` - List of etcd endpoints
    /// * `prefix` - Optional key prefix for namespacing
    /// * `priority` - Priority for this source (higher values override lower values)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::adapters::EtcdAdapter;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let adapter = EtcdAdapter::with_priority(
    ///     vec!["localhost:2379"],
    ///     Some("myapp/"),
    ///     2
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn with_priority<S: AsRef<str>>(
        endpoints: Vec<S>,
        prefix: Option<&str>,
        priority: u8,
    ) -> Result<Self> {
        let mut adapter = Self::new(endpoints, prefix).await?;
        adapter.priority = priority;
        Ok(adapter)
    }

    /// Loads all keys from etcd into the cache.
    async fn load_all_keys(&mut self) -> Result<()> {
        let prefix = self.prefix.as_deref().unwrap_or("");

        let mut client = (*self.client).clone();

        let options = GetOptions::new().with_prefix();
        let response =
            client
                .get(prefix, Some(options))
                .await
                .map_err(|e| ConfigError::SourceError {
                    source_name: "etcd".to_string(),
                    message: format!("Failed to fetch keys from etcd: {}", e),
                    source: Some(Box::new(e)),
                })?;

        self.cache.clear();

        for kv in response.kvs() {
            if let (Ok(key), Ok(value)) = (kv.key_str(), kv.value_str()) {
                // Strip prefix from key
                let key = if !prefix.is_empty() && key.starts_with(prefix) {
                    &key[prefix.len()..]
                } else {
                    key
                };

                // Convert slashes to dots for consistency with other adapters
                let key = key.replace('/', ".");

                self.cache.insert(key, value.to_string());
            }
        }

        Ok(())
    }

    /// Reloads all keys from etcd synchronously.
    ///
    /// Note: This method uses a shared runtime to perform async operations efficiently.
    /// If called from an async context, it will spawn a separate thread to avoid blocking.
    fn reload_sync(&mut self) -> Result<()> {
        let endpoints = self.endpoints.clone();
        let prefix = self.prefix.clone();

        // Try to use the current runtime if available, otherwise use the shared runtime
        let new_cache = if tokio::runtime::Handle::try_current().is_ok() {
            // We're in an async context, need to spawn a separate thread with the shared runtime
            // to avoid blocking the current runtime's executor
            let handle = std::thread::spawn(move || {

                RELOAD_RUNTIME.block_on(async move {
                    let prefix_str = prefix.as_deref().unwrap_or("");

                    // Connect fresh to etcd
                    let mut client = Client::connect(&endpoints, None)
                        .await
                        .map_err(|e| ConfigError::SourceError {
                            source_name: "etcd".to_string(),
                            message: format!("Failed to connect to etcd: {}", e),
                            source: Some(Box::new(e)),
                        })?;

                    let options = GetOptions::new().with_prefix();
                    let response = client.get(prefix_str, Some(options)).await.map_err(|e| {
                        ConfigError::SourceError {
                            source_name: "etcd".to_string(),
                            message: format!("Failed to fetch keys from etcd: {}", e),
                            source: Some(Box::new(e)),
                        }
                    })?;

                    let mut new_cache = HashMap::new();

                    for kv in response.kvs() {
                        if let (Ok(key), Ok(value)) = (kv.key_str(), kv.value_str()) {
                            // Strip prefix from key
                            let key = if !prefix_str.is_empty() && key.starts_with(prefix_str) {
                                &key[prefix_str.len()..]
                            } else {
                                key
                            };

                            // Convert slashes to dots for consistency
                            let key = key.replace('/', ".");

                            new_cache.insert(key, value.to_string());
                        }
                    }

                    Ok::<HashMap<String, String>, ConfigError>(new_cache)
                })
            });

            handle
                .join()
                .map_err(|_| ConfigError::SourceError {
                    source_name: "etcd".to_string(),
                    message: "Failed to join reload thread".to_string(),
                    source: None,
                })?
        } else {
            // No runtime available, use the shared runtime
            RELOAD_RUNTIME.block_on(async move {
                let prefix_str = prefix.as_deref().unwrap_or("");

                // Connect fresh to etcd
                let mut client = Client::connect(&endpoints, None)
                    .await
                    .map_err(|e| ConfigError::SourceError {
                        source_name: "etcd".to_string(),
                        message: format!("Failed to connect to etcd: {}", e),
                        source: Some(Box::new(e)),
                    })?;

                let options = GetOptions::new().with_prefix();
                let response = client.get(prefix_str, Some(options)).await.map_err(|e| {
                    ConfigError::SourceError {
                        source_name: "etcd".to_string(),
                        message: format!("Failed to fetch keys from etcd: {}", e),
                        source: Some(Box::new(e)),
                    }
                })?;

                let mut new_cache = HashMap::new();

                for kv in response.kvs() {
                    if let (Ok(key), Ok(value)) = (kv.key_str(), kv.value_str()) {
                        // Strip prefix from key
                        let key = if !prefix_str.is_empty() && key.starts_with(prefix_str) {
                            &key[prefix_str.len()..]
                        } else {
                            key
                        };

                        // Convert slashes to dots for consistency
                        let key = key.replace('/', ".");

                        new_cache.insert(key, value.to_string());
                    }
                }

                Ok::<HashMap<String, String>, ConfigError>(new_cache)
            })
        }?;

        self.cache = new_cache;
        Ok(())
    }
}

impl ConfigSource for EtcdAdapter {
    fn name(&self) -> &str {
        "etcd"
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> {
        Ok(self
            .cache
            .get(key.as_str())
            .map(|v| ConfigValue::from(v.as_str())))
    }

    fn all_keys(&self) -> Result<Vec<ConfigKey>> {
        Ok(self
            .cache
            .keys()
            .map(|k| ConfigKey::from(k.as_str()))
            .collect())
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_sync()
    }
}

// Tests for etcd adapter are in tests/etcd_integration_tests.rs
// (requires Docker to run)
