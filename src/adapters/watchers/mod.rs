// SPDX-License-Identifier: MIT OR Apache-2.0

//! Watcher implementations for configuration change detection.
//!
//! This module contains implementations of the `ConfigWatcher` trait for
//! monitoring configuration changes from various sources.

#[cfg(feature = "reload")]
pub mod file_watcher;

#[cfg(feature = "reload")]
pub use file_watcher::FileWatcher;

#[cfg(feature = "etcd")]
pub mod etcd_watcher;

#[cfg(feature = "etcd")]
pub use etcd_watcher::EtcdWatcher;

#[cfg(feature = "redis")]
pub mod redis_watcher;

#[cfg(feature = "redis")]
pub use redis_watcher::RedisWatcher;
