// SPDX-License-Identifier: MIT OR Apache-2.0

//! Redis configuration source adapter.
//!
//! This module provides an adapter that reads configuration values from Redis.

use crate::domain::{ConfigError, ConfigKey, ConfigValue, Result};
use crate::ports::ConfigSource;
use once_cell::sync::Lazy;
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use std::collections::HashMap;
use std::sync::Arc;

/// Shared runtime for reload operations to avoid expensive runtime creation on every reload
static RELOAD_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Runtime::new().expect("Failed to create reload runtime for Redis adapter")
});

/// Storage mode for Redis configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedisStorageMode {
    /// Store each configuration key as a separate Redis key with a prefix.
    /// Example: prefix:database.host, prefix:database.port
    StringKeys,
    /// Store all configuration in a single Redis hash.
    /// Example: HGETALL config_hash
    Hash,
}

/// Configuration source adapter for Redis.
///
/// This adapter reads configuration values from a Redis instance. It supports
/// both string key storage (with prefix) and hash storage modes.
///
/// # Priority
///
/// Redis has a default priority of 1, but this can be customized.
///
/// # Examples
///
/// ```rust,no_run
/// use configuration::adapters::{RedisAdapter, RedisStorageMode};
/// use configuration::ports::ConfigSource;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Connect to Redis using string keys with prefix
/// let adapter = RedisAdapter::new(
///     "redis://localhost:6379",
///     "myapp:",
///     RedisStorageMode::StringKeys
/// ).await?;
///
/// // Or use hash storage
/// let adapter = RedisAdapter::new(
///     "redis://localhost:6379",
///     "myapp:config",
///     RedisStorageMode::Hash
/// ).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct RedisAdapter {
    /// Redis client
    client: Arc<Client>,
    /// Key prefix or hash key name
    namespace: String,
    /// Storage mode (string keys or hash)
    storage_mode: RedisStorageMode,
    /// Priority for this source
    priority: u8,
    /// Cached configuration values
    cache: HashMap<String, String>,
}

impl RedisAdapter {
    /// Validates namespace to prevent injection attacks
    fn validate_namespace(namespace: &str) -> Result<()> {
        // Disallow wildcard characters and other special Redis pattern characters
        if namespace.contains(['*', '?', '[', ']', '\\']) {
            return Err(ConfigError::SourceError {
                source_name: "redis".to_string(),
                message: "Namespace contains invalid characters (* ? [ ] \\)".to_string(),
                source: None,
            });
        }
        Ok(())
    }

    /// Creates a new Redis adapter with the given connection URL.
    ///
    /// # Arguments
    ///
    /// * `url` - Redis connection URL (e.g., `"redis://localhost:6379"`)
    /// * `namespace` - Key prefix (for StringKeys mode) or hash key name (for Hash mode)
    /// * `storage_mode` - Whether to use string keys or hash storage
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::adapters::{RedisAdapter, RedisStorageMode};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let adapter = RedisAdapter::new(
    ///     "redis://localhost:6379",
    ///     "myapp:",
    ///     RedisStorageMode::StringKeys
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(url: &str, namespace: &str, storage_mode: RedisStorageMode) -> Result<Self> {
        // Validate namespace to prevent injection attacks
        Self::validate_namespace(namespace)?;

        let client = Client::open(url).map_err(|e| ConfigError::SourceError {
            source_name: "redis".to_string(),
            message: format!("Failed to create Redis client: {}", e),
            source: Some(Box::new(e)),
        })?;

        let mut adapter = Self {
            client: Arc::new(client),
            namespace: namespace.to_string(),
            storage_mode,
            priority: 1,
            cache: HashMap::new(),
        };

        // Initial load of all keys
        adapter.load_all_keys().await?;

        Ok(adapter)
    }

    /// Creates a new Redis adapter with a custom priority.
    ///
    /// # Arguments
    ///
    /// * `url` - Redis connection URL
    /// * `namespace` - Key prefix or hash key name
    /// * `storage_mode` - Whether to use string keys or hash storage
    /// * `priority` - Priority for this source (higher values override lower values)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::adapters::{RedisAdapter, RedisStorageMode};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let adapter = RedisAdapter::with_priority(
    ///     "redis://localhost:6379",
    ///     "myapp:",
    ///     RedisStorageMode::StringKeys,
    ///     2
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn with_priority(
        url: &str,
        namespace: &str,
        storage_mode: RedisStorageMode,
        priority: u8,
    ) -> Result<Self> {
        let mut adapter = Self::new(url, namespace, storage_mode).await?;
        adapter.priority = priority;
        Ok(adapter)
    }

    /// Gets a multiplexed connection to Redis.
    async fn get_connection(&self) -> Result<MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ConfigError::SourceError {
                source_name: "redis".to_string(),
                message: format!("Failed to connect to Redis: {}", e),
                source: Some(Box::new(e)),
            })
    }

    /// Loads all keys from Redis into the cache.
    async fn load_all_keys(&mut self) -> Result<()> {
        let mut conn = self.get_connection().await?;

        self.cache.clear();

        match self.storage_mode {
            RedisStorageMode::Hash => {
                // Load all fields from hash
                let hash: HashMap<String, String> =
                    conn.hgetall(&self.namespace)
                        .await
                        .map_err(|e| ConfigError::SourceError {
                            source_name: "redis".to_string(),
                            message: format!("Failed to fetch hash from Redis: {}", e),
                            source: Some(Box::new(e)),
                        })?;

                self.cache = hash;
            }
            RedisStorageMode::StringKeys => {
                // Use SCAN instead of KEYS to avoid blocking the Redis server
                let pattern = format!("{}*", self.namespace);
                let mut cursor: u64 = 0;
                let mut all_keys = Vec::new();

                loop {
                    let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                        .arg(cursor)
                        .arg("MATCH")
                        .arg(&pattern)
                        .arg("COUNT")
                        .arg(100)
                        .query_async(&mut conn)
                        .await
                        .map_err(|e| ConfigError::SourceError {
                            source_name: "redis".to_string(),
                            message: format!("Failed to scan keys from Redis: {}", e),
                            source: Some(Box::new(e)),
                        })?;

                    all_keys.extend(keys);
                    cursor = new_cursor;
                    if cursor == 0 {
                        break;
                    }
                }

                // Fetch all values
                for key in all_keys {
                    let value: String =
                        conn.get(&key).await.map_err(|e| ConfigError::SourceError {
                            source_name: "redis".to_string(),
                            message: format!("Failed to fetch value from Redis: {}", e),
                            source: Some(Box::new(e)),
                        })?;

                    // Strip prefix from key
                    let key = if key.starts_with(&self.namespace) {
                        &key[self.namespace.len()..]
                    } else {
                        &key
                    };

                    self.cache.insert(key.to_string(), value);
                }
            }
        }

        Ok(())
    }

    /// Reloads all keys from Redis synchronously.
    ///
    /// Note: This method uses a shared runtime to perform async operations efficiently.
    /// If called from an async context, it will spawn a separate thread to avoid blocking.
    fn reload_sync(&mut self) -> Result<()> {
        let client = Arc::clone(&self.client);
        let namespace = self.namespace.clone();
        let storage_mode = self.storage_mode;

        // Try to use the current runtime if available, otherwise use the shared runtime
        let new_cache = if tokio::runtime::Handle::try_current().is_ok() {
            // We're in an async context, need to spawn a separate thread with the shared runtime
            // to avoid blocking the current runtime's executor
            let handle = std::thread::spawn(move || {

                RELOAD_RUNTIME.block_on(async move {
                    let mut conn = client
                        .get_multiplexed_async_connection()
                        .await
                        .map_err(|e| ConfigError::SourceError {
                            source_name: "redis".to_string(),
                            message: format!("Failed to connect to Redis: {}", e),
                            source: Some(Box::new(e)),
                        })?;

                    let mut new_cache = HashMap::new();

                    match storage_mode {
                        RedisStorageMode::Hash => {
                            // Load all fields from hash
                            let hash: HashMap<String, String> = conn
                                .hgetall(&namespace)
                                .await
                                .map_err(|e| ConfigError::SourceError {
                                    source_name: "redis".to_string(),
                                    message: format!("Failed to fetch hash from Redis: {}", e),
                                    source: Some(Box::new(e)),
                                })?;

                            new_cache = hash;
                        }
                        RedisStorageMode::StringKeys => {
                            // Use SCAN instead of KEYS to avoid blocking the Redis server
                            let pattern = format!("{}*", namespace);
                            let mut cursor: u64 = 0;
                            let mut all_keys = Vec::new();

                            loop {
                                let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                                    .arg(cursor)
                                    .arg("MATCH")
                                    .arg(&pattern)
                                    .arg("COUNT")
                                    .arg(100)
                                    .query_async(&mut conn)
                                    .await
                                    .map_err(|e| ConfigError::SourceError {
                                        source_name: "redis".to_string(),
                                        message: format!("Failed to scan keys from Redis: {}", e),
                                        source: Some(Box::new(e)),
                                    })?;

                                all_keys.extend(keys);
                                cursor = new_cursor;
                                if cursor == 0 {
                                    break;
                                }
                            }

                            // Fetch all values
                            for key in all_keys {
                                let value: String = conn.get(&key).await.map_err(|e| {
                                    ConfigError::SourceError {
                                        source_name: "redis".to_string(),
                                        message: format!("Failed to fetch value from Redis: {}", e),
                                        source: Some(Box::new(e)),
                                    }
                                })?;

                                // Strip prefix from key
                                let key = if key.starts_with(&namespace) {
                                    &key[namespace.len()..]
                                } else {
                                    &key
                                };

                                new_cache.insert(key.to_string(), value);
                            }
                        }
                    }

                    Ok::<HashMap<String, String>, ConfigError>(new_cache)
                })
            });

            handle
                .join()
                .map_err(|_| ConfigError::SourceError {
                    source_name: "redis".to_string(),
                    message: "Failed to join reload thread".to_string(),
                    source: None,
                })?
        } else {
            // No runtime available, use the shared runtime
            RELOAD_RUNTIME.block_on(async move {
                let mut conn = client
                    .get_multiplexed_async_connection()
                    .await
                    .map_err(|e| ConfigError::SourceError {
                        source_name: "redis".to_string(),
                        message: format!("Failed to connect to Redis: {}", e),
                        source: Some(Box::new(e)),
                    })?;

                let mut new_cache = HashMap::new();

                match storage_mode {
                    RedisStorageMode::Hash => {
                        // Load all fields from hash
                        let hash: HashMap<String, String> = conn
                            .hgetall(&namespace)
                            .await
                            .map_err(|e| ConfigError::SourceError {
                                source_name: "redis".to_string(),
                                message: format!("Failed to fetch hash from Redis: {}", e),
                                source: Some(Box::new(e)),
                            })?;

                        new_cache = hash;
                    }
                    RedisStorageMode::StringKeys => {
                        // Use SCAN instead of KEYS to avoid blocking the Redis server
                        let pattern = format!("{}*", namespace);
                        let mut cursor: u64 = 0;
                        let mut all_keys = Vec::new();

                        loop {
                            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                                .arg(cursor)
                                .arg("MATCH")
                                .arg(&pattern)
                                .arg("COUNT")
                                .arg(100)
                                .query_async(&mut conn)
                                .await
                                .map_err(|e| ConfigError::SourceError {
                                    source_name: "redis".to_string(),
                                    message: format!("Failed to scan keys from Redis: {}", e),
                                    source: Some(Box::new(e)),
                                })?;

                            all_keys.extend(keys);
                            cursor = new_cursor;
                            if cursor == 0 {
                                break;
                            }
                        }

                        // Fetch all values
                        for key in all_keys {
                            let value: String =
                                conn.get(&key).await.map_err(|e| ConfigError::SourceError {
                                    source_name: "redis".to_string(),
                                    message: format!("Failed to fetch value from Redis: {}", e),
                                    source: Some(Box::new(e)),
                                })?;

                            // Strip prefix from key
                            let key = if key.starts_with(&namespace) {
                                &key[namespace.len()..]
                            } else {
                                &key
                            };

                            new_cache.insert(key.to_string(), value);
                        }
                    }
                }

                Ok::<HashMap<String, String>, ConfigError>(new_cache)
            })
        }?;

        self.cache = new_cache;
        Ok(())
    }
}

impl ConfigSource for RedisAdapter {
    fn name(&self) -> &str {
        "redis"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_storage_modes() {
        // Test that both storage modes are available
        assert_eq!(RedisStorageMode::StringKeys, RedisStorageMode::StringKeys);
        assert_eq!(RedisStorageMode::Hash, RedisStorageMode::Hash);
        assert_ne!(RedisStorageMode::StringKeys, RedisStorageMode::Hash);
    }
}
