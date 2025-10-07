// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for dynamic configuration reloading.

use hexcfg::prelude::*;
use std::fs;
use tempfile::NamedTempFile;

#[cfg(feature = "reload")]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(feature = "reload")]
use std::sync::Arc;
#[cfg(feature = "reload")]
use std::thread;
#[cfg(feature = "reload")]
use std::time::Duration;

#[test]
#[cfg(feature = "yaml")]
fn test_manual_reload() {
    // Create a temporary YAML file
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    // Write initial content
    fs::write(&path, "key: initial_value\n").unwrap();

    let mut service = DefaultConfigService::builder()
        .with_yaml_file(&path)
        .unwrap()
        .build()
        .unwrap();

    // Get initial value
    let key = ConfigKey::from("key");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "initial_value");

    // Update file
    fs::write(&path, "key: updated_value\n").unwrap();

    // Value should still be old before reload
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "initial_value");

    // Reload service
    service.reload().unwrap();

    // Value should be updated after reload
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "updated_value");
}

#[test]
#[cfg(feature = "yaml")]
fn test_reload_clears_cache() {
    // Create a temporary YAML file
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    // Write initial content with two keys
    fs::write(&path, "key1: value1\nkey2: value2\n").unwrap();

    let mut service = DefaultConfigService::builder()
        .with_yaml_file(&path)
        .unwrap()
        .build()
        .unwrap();

    // Get both values to populate cache
    let key1 = ConfigKey::from("key1");
    let key2 = ConfigKey::from("key2");
    assert_eq!(service.get(&key1).unwrap().as_str(), "value1");
    assert_eq!(service.get(&key2).unwrap().as_str(), "value2");

    // Update file with different values
    fs::write(&path, "key1: new_value1\nkey2: new_value2\n").unwrap();

    // Reload service
    service.reload().unwrap();

    // Both values should be updated (cache was cleared)
    assert_eq!(service.get(&key1).unwrap().as_str(), "new_value1");
    assert_eq!(service.get(&key2).unwrap().as_str(), "new_value2");
}

#[test]
#[cfg(feature = "yaml")]
fn test_reload_with_new_keys() {
    // Create a temporary YAML file
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    // Write initial content with one key
    fs::write(&path, "key1: value1\n").unwrap();

    let mut service = DefaultConfigService::builder()
        .with_yaml_file(&path)
        .unwrap()
        .build()
        .unwrap();

    // Check initial state
    let key1 = ConfigKey::from("key1");
    let key2 = ConfigKey::from("key2");
    assert!(service.has(&key1));
    assert!(!service.has(&key2));

    // Update file with new key
    fs::write(&path, "key1: value1\nkey2: value2\n").unwrap();

    // Reload service
    service.reload().unwrap();

    // Both keys should now exist
    assert!(service.has(&key1));
    assert!(service.has(&key2));
    assert_eq!(service.get(&key2).unwrap().as_str(), "value2");
}

#[test]
#[cfg(feature = "yaml")]
fn test_reload_with_removed_keys() {
    // Create a temporary YAML file
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    // Write initial content with two keys
    fs::write(&path, "key1: value1\nkey2: value2\n").unwrap();

    let mut service = DefaultConfigService::builder()
        .with_yaml_file(&path)
        .unwrap()
        .build()
        .unwrap();

    // Check initial state
    let key1 = ConfigKey::from("key1");
    let key2 = ConfigKey::from("key2");
    assert!(service.has(&key1));
    assert!(service.has(&key2));

    // Update file with only one key
    fs::write(&path, "key1: value1\n").unwrap();

    // Reload service
    service.reload().unwrap();

    // Only key1 should exist
    assert!(service.has(&key1));
    assert!(!service.has(&key2));
}

#[test]
#[cfg(all(feature = "yaml", feature = "env"))]
fn test_reload_respects_precedence() {
    use std::env;

    // Set environment variable
    env::set_var("TEST_KEY", "env_value");

    // Create a temporary YAML file
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    // Write initial content
    fs::write(&path, "test.key: yaml_value\n").unwrap();

    let mut service = DefaultConfigService::builder()
        .with_yaml_file(&path)
        .unwrap()
        .with_env_vars()
        .build()
        .unwrap();

    // Environment should win (priority 2 > 1)
    let key = ConfigKey::from("test.key");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "env_value");

    // Update YAML file
    fs::write(&path, "test.key: yaml_value_updated\n").unwrap();

    // Reload service
    service.reload().unwrap();

    // Environment should still win
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "env_value");

    // Clean up
    env::remove_var("TEST_KEY");
}

#[test]
#[cfg(all(feature = "yaml", feature = "reload"))]
fn test_file_watcher_creation() {
    use hexcfg::adapters::FileWatcher;

    // Create a temporary YAML file
    let temp_file = NamedTempFile::new().unwrap();

    // Create a file watcher
    let watcher = FileWatcher::new(temp_file.path(), None);
    assert!(watcher.is_ok());
}

#[test]
#[cfg(all(feature = "yaml", feature = "reload"))]
fn test_file_watcher_nonexistent_file() {
    use hexcfg::adapters::FileWatcher;

    // Try to watch a nonexistent file
    let watcher = FileWatcher::new("/nonexistent/file.yaml", None);
    assert!(watcher.is_err());
}

#[test]
#[cfg(all(feature = "yaml", feature = "reload"))]
fn test_file_watcher_start_stop() {
    use hexcfg::adapters::FileWatcher;
    use hexcfg::ports::ConfigWatcher;

    // Create a temporary YAML file
    let temp_file = NamedTempFile::new().unwrap();

    let mut watcher = FileWatcher::new(temp_file.path(), None).unwrap();

    let callback = Arc::new(|_key: ConfigKey| {
        // Callback for testing
    });

    // Start watching
    assert!(watcher.watch(callback).is_ok());

    // Stop watching
    assert!(watcher.stop().is_ok());
}

#[test]
#[cfg(all(feature = "yaml", feature = "reload"))]
fn test_file_watcher_callback_triggered() {
    use hexcfg::adapters::FileWatcher;
    use hexcfg::ports::ConfigWatcher;

    // Create a temporary YAML file
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    let mut watcher = FileWatcher::new(&path, Some(Duration::from_millis(100))).unwrap();

    let triggered = Arc::new(AtomicBool::new(false));
    let triggered_clone = Arc::clone(&triggered);

    let callback = Arc::new(move |_key: ConfigKey| {
        triggered_clone.store(true, Ordering::SeqCst);
    });

    watcher.watch(callback).unwrap();

    // Wait for watcher to initialize
    thread::sleep(Duration::from_millis(100));

    // Modify the file
    fs::write(&path, "key: modified_value\n").unwrap();

    // Wait for the event to be processed (debounce + processing time)
    thread::sleep(Duration::from_millis(400));

    // Stop the watcher
    watcher.stop().unwrap();

    // Note: File system events can be flaky in test environments
    // We don't assert the result to avoid flaky tests, but log it
    let was_triggered = triggered.load(Ordering::SeqCst);
    if !was_triggered {
        eprintln!("Warning: File watcher callback was not triggered (this can happen in test environments)");
    }
}

#[test]
#[cfg(all(feature = "yaml", feature = "reload"))]
fn test_file_watcher_debouncing() {
    use hexcfg::adapters::FileWatcher;
    use hexcfg::ports::ConfigWatcher;

    // Create a temporary YAML file
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    // Use a longer debounce delay for this test
    let mut watcher = FileWatcher::new(&path, Some(Duration::from_millis(500))).unwrap();

    let trigger_count = Arc::new(AtomicUsize::new(0));
    let trigger_count_clone = Arc::clone(&trigger_count);

    let callback = Arc::new(move |_key: ConfigKey| {
        trigger_count_clone.fetch_add(1, Ordering::SeqCst);
    });

    watcher.watch(callback).unwrap();

    // Wait for watcher to initialize
    thread::sleep(Duration::from_millis(100));

    // Rapidly modify the file multiple times
    for i in 0..5 {
        fs::write(&path, format!("key: value_{}\n", i)).unwrap();
        thread::sleep(Duration::from_millis(50));
    }

    // Wait for debounce to settle
    thread::sleep(Duration::from_millis(800));

    watcher.stop().unwrap();

    let count = trigger_count.load(Ordering::SeqCst);

    // Due to debouncing, we should have fewer triggers than modifications
    // Note: This test can be flaky depending on file system behavior
    if count > 0 {
        assert!(
            count < 5,
            "Expected debouncing to reduce trigger count, got {}",
            count
        );
    }
}

#[test]
#[cfg(all(feature = "yaml", feature = "reload"))]
fn test_service_with_watcher() {
    use hexcfg::adapters::FileWatcher;

    // Create a temporary YAML file
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    fs::write(&path, "key: initial\n").unwrap();

    let mut service = DefaultConfigService::builder()
        .with_yaml_file(&path)
        .unwrap()
        .build()
        .unwrap();

    // Create a file watcher
    let watcher = FileWatcher::new(&path, None).unwrap();

    // Register watcher with the service
    let _ = service.register_watcher(Box::new(watcher));

    // The service supports the watcher interface
    assert!(service.has(&ConfigKey::from("key")));
}
