// SPDX-License-Identifier: MIT OR Apache-2.0

//! Redis configuration watcher.
//!
//! This module provides a watcher that monitors Redis for configuration changes
//! using Redis keyspace notifications (pub/sub).

use crate::domain::{ConfigError, ConfigKey, Result};
use crate::ports::{ChangeCallback, ConfigWatcher};
use redis::Client;
use std::sync::mpsc::{channel, Sender};
use std::thread::{self, JoinHandle};

/// Watcher for Redis configuration changes.
///
/// This watcher uses Redis keyspace notifications to receive real-time notifications
/// when configuration values change. It requires Redis keyspace notifications to be
/// enabled on the server (`notify-keyspace-events` config).
///
/// **Note**: Redis keyspace notifications must be enabled. Set in redis.conf:
/// ```text
/// notify-keyspace-events KEA
/// ```
/// Or via CLI: `CONFIG SET notify-keyspace-events KEA`
///
/// # Examples
///
/// ```rust,no_run
/// use configuration::adapters::RedisWatcher;
/// use configuration::ports::ConfigWatcher;
/// use std::sync::Arc;
///
/// # fn main() -> configuration::domain::Result<()> {
/// let mut watcher = RedisWatcher::new(
///     "redis://localhost:6379",
///     "myapp:"
/// )?;
///
/// watcher.watch(Arc::new(|key| {
///     println!("Configuration changed: {}", key);
/// }))?;
///
/// // Later, stop watching
/// watcher.stop()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct RedisWatcher {
    /// Redis connection URL
    url: String,
    /// Key prefix/pattern to watch
    namespace: String,
    /// Stop signal sender
    stop_tx: Option<Sender<()>>,
    /// Watch thread handle
    watch_thread: Option<JoinHandle<()>>,
}

impl RedisWatcher {
    /// Creates a new Redis watcher.
    ///
    /// **Important**: This requires Redis keyspace notifications to be enabled.
    /// Configure Redis with: `CONFIG SET notify-keyspace-events KEA`
    ///
    /// # Arguments
    ///
    /// * `url` - Redis connection URL (e.g., `"redis://localhost:6379"`)
    /// * `namespace` - Key prefix to watch (e.g., `"myapp:"`)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::adapters::RedisWatcher;
    ///
    /// # fn main() -> configuration::domain::Result<()> {
    /// let watcher = RedisWatcher::new(
    ///     "redis://localhost:6379",
    ///     "myapp:"
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(url: &str, namespace: &str) -> Result<Self> {
        // Test connection
        let client = Client::open(url).map_err(|e| ConfigError::WatcherError {
            message: format!("Failed to create Redis client: {}", e),
            source: Some(Box::new(e)),
        })?;

        // Test connection
        let _conn = client
            .get_connection()
            .map_err(|e| ConfigError::WatcherError {
                message: format!("Failed to connect to Redis: {}", e),
                source: Some(Box::new(e)),
            })?;

        tracing::debug!("Redis watcher initialized. Note: keyspace notifications must be enabled manually with: CONFIG SET notify-keyspace-events KEA");

        Ok(Self {
            url: url.to_string(),
            namespace: namespace.to_string(),
            stop_tx: None,
            watch_thread: None,
        })
    }

    /// Attempts to enable keyspace notifications if they're not already enabled.
    ///
    /// This method tries to set `notify-keyspace-events` to `KEA` (Keyspace events,
    /// All commands). This requires appropriate permissions on the Redis server.
    ///
    /// Returns `Ok(())` if successful or if already enabled.
    pub fn try_enable_keyspace_notifications(&self) -> Result<()> {
        let client = Client::open(self.url.as_str()).map_err(|e| ConfigError::WatcherError {
            message: format!("Failed to create Redis client: {}", e),
            source: Some(Box::new(e)),
        })?;

        let mut conn = client
            .get_connection()
            .map_err(|e| ConfigError::WatcherError {
                message: format!("Failed to connect to Redis: {}", e),
                source: Some(Box::new(e)),
            })?;

        redis::cmd("CONFIG")
            .arg("SET")
            .arg("notify-keyspace-events")
            .arg("KEA")
            .query::<()>(&mut conn)
            .map_err(|e| ConfigError::WatcherError {
                message: format!(
                    "Failed to enable keyspace notifications. Enable manually with: CONFIG SET notify-keyspace-events KEA. Error: {}",
                    e
                ),
                source: Some(Box::new(e)),
            })?;

        tracing::info!("Enabled Redis keyspace notifications");
        Ok(())
    }
}

impl ConfigWatcher for RedisWatcher {
    fn watch(&mut self, callback: ChangeCallback) -> Result<()> {
        if self.watch_thread.is_some() {
            return Err(ConfigError::WatcherError {
                message: "Watcher is already running".to_string(),
                source: None,
            });
        }

        let (stop_tx, stop_rx) = channel();
        self.stop_tx = Some(stop_tx);

        let url = self.url.clone();
        let namespace = self.namespace.clone();

        let watch_thread = thread::spawn(move || {
            loop {
                // Check for stop signal
                if stop_rx.try_recv().is_ok() {
                    tracing::debug!("Redis watcher stopping");
                    break;
                }

                // Connect to Redis
                let client = match Client::open(url.as_str()) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Failed to create Redis client for watching: {}", e);
                        thread::sleep(std::time::Duration::from_secs(5));
                        continue;
                    }
                };

                let mut conn = match client.get_connection() {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Failed to connect to Redis for watching: {}", e);
                        thread::sleep(std::time::Duration::from_secs(5));
                        continue;
                    }
                };

                // Subscribe to keyspace notifications for keys with our prefix
                // Pattern: __keyspace@0__:namespace*
                let pattern = format!("__keyspace@0__:{}*", namespace);
                tracing::info!("Starting Redis watch on pattern: {}", pattern);

                let mut pubsub = conn.as_pubsub();
                if let Err(e) = pubsub.psubscribe(&pattern) {
                    tracing::error!("Failed to subscribe to Redis keyspace events: {}. Ensure keyspace notifications are enabled with: CONFIG SET notify-keyspace-events KEA", e);
                    thread::sleep(std::time::Duration::from_secs(5));
                    continue;
                }

                // Process messages
                loop {
                    // Check for stop signal
                    if stop_rx.try_recv().is_ok() {
                        tracing::debug!("Redis watcher stopping");
                        return;
                    }

                    // Set a timeout to periodically check stop signal
                    pubsub
                        .set_read_timeout(Some(std::time::Duration::from_millis(100)))
                        .ok();

                    match pubsub.get_message() {
                        Ok(msg) => {
                            let channel: String = msg.get_channel_name().to_string();

                            // Extract key from channel name: __keyspace@0__:namespace:key
                            if let Some(key_with_namespace) =
                                channel.strip_prefix("__keyspace@0__:")
                            {
                                // Strip namespace prefix
                                let key = if key_with_namespace.starts_with(&namespace) {
                                    &key_with_namespace[namespace.len()..]
                                } else {
                                    key_with_namespace
                                };

                                tracing::debug!("Redis key changed: {}", key);
                                callback(ConfigKey::from(key.to_string()));
                            }
                        }
                        Err(e) => {
                            // Timeout errors are expected when checking stop signal
                            if e.is_timeout() {
                                continue;
                            }
                            tracing::error!("Redis pub/sub error: {}", e);
                            break; // Reconnect
                        }
                    }
                }
            }
        });

        self.watch_thread = Some(watch_thread);

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        // Send stop signal
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }

        // Wait for the thread to finish
        if let Some(handle) = self.watch_thread.take() {
            handle.join().map_err(|_| ConfigError::WatcherError {
                message: "Failed to join Redis watcher thread".to_string(),
                source: None,
            })?;
        }

        Ok(())
    }
}

impl Drop for RedisWatcher {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
