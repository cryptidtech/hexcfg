// SPDX-License-Identifier: MIT OR Apache-2.0

//! Watcher implementations for configuration change detection.
//!
//! This module contains implementations of the `ConfigWatcher` trait for
//! monitoring configuration changes from various sources.

#[cfg(feature = "reload")]
pub mod file_watcher;

#[cfg(feature = "reload")]
pub use file_watcher::FileWatcher;
