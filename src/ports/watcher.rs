// SPDX-License-Identifier: MIT OR Apache-2.0

//! Configuration watcher trait definition.
//!
//! This module defines the `ConfigWatcher` trait, which provides an interface for
//! watching configuration sources for changes and triggering callbacks when changes occur.

use crate::domain::{ConfigKey, Result};
use std::sync::Arc;

/// Type alias for change notification callbacks.
///
/// This callback is invoked when a configuration value changes. It receives the
/// key that changed as a parameter.
pub type ChangeCallback = Arc<dyn Fn(ConfigKey) + Send + Sync>;

/// A trait for watching configuration sources for changes.
///
/// This trait defines the interface for implementing configuration watchers that can
/// monitor sources (files, remote services, etc.) for changes and trigger callbacks
/// when changes are detected.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow for use in multi-threaded contexts.
///
/// # Examples
///
/// ```rust
/// use hexcfg::ports::ConfigWatcher;
/// use hexcfg::domain::{ConfigKey, Result};
/// use std::sync::Arc;
///
/// struct MyWatcher;
///
/// impl ConfigWatcher for MyWatcher {
///     fn watch(&mut self, callback: Arc<dyn Fn(ConfigKey) + Send + Sync>) -> Result<()> {
///         // Implementation here
///         Ok(())
///     }
///
///     fn stop(&mut self) -> Result<()> {
///         Ok(())
///     }
/// }
/// ```
pub trait ConfigWatcher: Send + Sync {
    /// Starts watching for configuration changes.
    ///
    /// When a change is detected, the provided callback will be invoked with the
    /// configuration key that changed. The callback should be non-blocking to avoid
    /// delaying the watcher.
    ///
    /// # Arguments
    ///
    /// * `callback` - A function to call when a configuration change is detected
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The watcher was successfully started
    /// * `Err(ConfigError)` - An error occurred while starting the watcher
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hexcfg::ports::ConfigWatcher;
    /// # use hexcfg::domain::{ConfigKey, Result};
    /// # use std::sync::Arc;
    /// # struct MyWatcher;
    /// # impl ConfigWatcher for MyWatcher {
    /// #     fn watch(&mut self, callback: Arc<dyn Fn(ConfigKey) + Send + Sync>) -> Result<()> {
    /// #         Ok(())
    /// #     }
    /// #     fn stop(&mut self) -> Result<()> { Ok(()) }
    /// # }
    /// let mut watcher = MyWatcher;
    /// let callback = Arc::new(|key: ConfigKey| {
    ///     println!("Config changed: {}", key);
    /// });
    /// watcher.watch(callback).unwrap();
    /// ```
    fn watch(&mut self, callback: ChangeCallback) -> Result<()>;

    /// Stops watching for configuration changes.
    ///
    /// After calling this method, no more change notifications will be sent.
    /// This method should clean up any resources used by the watcher.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The watcher was successfully stopped
    /// * `Err(ConfigError)` - An error occurred while stopping the watcher
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hexcfg::ports::ConfigWatcher;
    /// # use hexcfg::domain::{ConfigKey, Result};
    /// # use std::sync::Arc;
    /// # struct MyWatcher;
    /// # impl ConfigWatcher for MyWatcher {
    /// #     fn watch(&mut self, callback: Arc<dyn Fn(ConfigKey) + Send + Sync>) -> Result<()> {
    /// #         Ok(())
    /// #     }
    /// #     fn stop(&mut self) -> Result<()> { Ok(()) }
    /// # }
    /// let mut watcher = MyWatcher;
    /// watcher.stop().unwrap();
    /// ```
    fn stop(&mut self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test implementation of ConfigWatcher for testing purposes
    struct TestWatcher {
        is_watching: bool,
    }

    impl TestWatcher {
        fn new() -> Self {
            TestWatcher { is_watching: false }
        }
    }

    impl ConfigWatcher for TestWatcher {
        fn watch(&mut self, _callback: ChangeCallback) -> Result<()> {
            self.is_watching = true;
            Ok(())
        }

        fn stop(&mut self) -> Result<()> {
            self.is_watching = false;
            Ok(())
        }
    }

    #[test]
    fn test_watcher_watch() {
        let mut watcher = TestWatcher::new();
        let callback = Arc::new(|_key: ConfigKey| {});
        assert!(watcher.watch(callback).is_ok());
        assert!(watcher.is_watching);
    }

    #[test]
    fn test_watcher_stop() {
        let mut watcher = TestWatcher::new();
        watcher.is_watching = true;
        assert!(watcher.stop().is_ok());
        assert!(!watcher.is_watching);
    }

    #[test]
    fn test_watcher_callback_invocation() {
        use std::sync::Mutex;

        let mut watcher = TestWatcher::new();
        let invoked = Arc::new(Mutex::new(false));
        let invoked_clone = invoked.clone();

        let callback = Arc::new(move |_key: ConfigKey| {
            *invoked_clone.lock().unwrap() = true;
        });

        watcher.watch(callback.clone()).unwrap();

        // Simulate a change notification
        callback(ConfigKey::from("test.key"));

        assert!(*invoked.lock().unwrap());
    }

    #[test]
    fn test_watcher_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn ConfigWatcher>>();
    }
}
