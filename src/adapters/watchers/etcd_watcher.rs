// SPDX-License-Identifier: MIT OR Apache-2.0

//! etcd configuration watcher.
//!
//! This module provides a watcher that monitors etcd for configuration changes
//! using etcd's native watch API.

use crate::domain::{ConfigError, ConfigKey, Result};
use crate::ports::{ChangeCallback, ConfigWatcher};
use etcd_client::{Client, WatchOptions};
use std::sync::mpsc::{channel, Sender};
use std::thread::{self, JoinHandle};

/// Watcher for etcd configuration changes.
///
/// This watcher uses etcd's native watch API to receive real-time notifications
/// when configuration values change. It monitors all keys with a specified prefix
/// and triggers callbacks when changes are detected.
///
/// # Examples
///
/// ```rust,no_run
/// use configuration::adapters::EtcdWatcher;
/// use configuration::ports::ConfigWatcher;
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() -> configuration::domain::Result<()> {
/// let mut watcher = EtcdWatcher::new(
///     vec!["localhost:2379"],
///     Some("myapp/")
/// ).await?;
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
pub struct EtcdWatcher {
    /// etcd endpoints
    endpoints: Vec<String>,
    /// Key prefix to watch
    prefix: Option<String>,
    /// Stop signal sender
    stop_tx: Option<Sender<()>>,
    /// Watch thread handle
    watch_thread: Option<JoinHandle<()>>,
}

impl EtcdWatcher {
    /// Creates a new etcd watcher.
    ///
    /// # Arguments
    ///
    /// * `endpoints` - List of etcd endpoints (e.g., `["localhost:2379"]`)
    /// * `prefix` - Optional key prefix to watch (e.g., `"myapp/"`)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::adapters::EtcdWatcher;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> configuration::domain::Result<()> {
    /// let watcher = EtcdWatcher::new(
    ///     vec!["localhost:2379"],
    ///     Some("myapp/")
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new<S: AsRef<str>>(endpoints: Vec<S>, prefix: Option<&str>) -> Result<Self> {
        let endpoints: Vec<String> = endpoints.iter().map(|s| s.as_ref().to_string()).collect();

        // Test connection
        let _client =
            Client::connect(&endpoints, None)
                .await
                .map_err(|e| ConfigError::WatcherError {
                    message: format!("Failed to connect to etcd: {}", e),
                    source: Some(Box::new(e)),
                })?;

        Ok(Self {
            endpoints,
            prefix: prefix.map(|s| s.to_string()),
            stop_tx: None,
            watch_thread: None,
        })
    }
}

impl ConfigWatcher for EtcdWatcher {
    fn watch(&mut self, callback: ChangeCallback) -> Result<()> {
        if self.watch_thread.is_some() {
            return Err(ConfigError::WatcherError {
                message: "Watcher is already running".to_string(),
                source: None,
            });
        }

        let (stop_tx, stop_rx) = channel();
        self.stop_tx = Some(stop_tx);

        let endpoints = self.endpoints.clone();
        let prefix = self.prefix.clone();

        let watch_thread = thread::spawn(move || {
            // Create a new runtime for this thread
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create tokio runtime for etcd watcher: {}", e);
                    return;
                }
            };

            runtime.block_on(async move {
                loop {
                    // Check for stop signal
                    if stop_rx.try_recv().is_ok() {
                        tracing::debug!("etcd watcher stopping");
                        break;
                    }

                    // Connect to etcd
                    let mut client = match Client::connect(&endpoints, None).await {
                        Ok(c) => c,
                        Err(e) => {
                            tracing::error!("Failed to connect to etcd for watching: {}", e);
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            continue;
                        }
                    };

                    let watch_prefix = prefix.as_deref().unwrap_or("");
                    tracing::info!("Starting etcd watch on prefix: {}", watch_prefix);

                    // Create watch stream
                    let options = WatchOptions::new().with_prefix();
                    let (mut _watcher, mut stream) = match client.watch(watch_prefix, Some(options)).await {
                        Ok((w, s)) => (w, s),
                        Err(e) => {
                            tracing::error!("Failed to create etcd watch: {}", e);
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            continue;
                        }
                    };

                    // Process watch events
                    loop {
                        // Check for stop signal
                        if stop_rx.try_recv().is_ok() {
                            tracing::debug!("etcd watcher stopping");
                            return;
                        }

                        tokio::select! {
                            Ok(resp) = stream.message() => {
                                if let Some(watch_resp) = resp {
                                    for event in watch_resp.events() {
                                        if let Some(kv) = event.kv() {
                                            if let Ok(key_str) = kv.key_str() {
                                                // Strip prefix from key
                                                let key = if !watch_prefix.is_empty() && key_str.starts_with(watch_prefix) {
                                                    &key_str[watch_prefix.len()..]
                                                } else {
                                                    key_str
                                                };

                                                // Convert slashes to dots for consistency
                                                let key = key.replace('/', ".");

                                                tracing::debug!("etcd key changed: {}", key);
                                                callback(ConfigKey::from(key));
                                            }
                                        }
                                    }
                                }
                            }
                            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                                // Timeout to check stop signal
                            }
                        }
                    }
                }
            });
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
                message: "Failed to join etcd watcher thread".to_string(),
                source: None,
            })?;
        }

        Ok(())
    }
}

impl Drop for EtcdWatcher {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
