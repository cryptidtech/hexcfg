// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for configuration source precedence.

use hexcfg::prelude::*;
use std::env;
use std::io::Write;
use tempfile::NamedTempFile;

/// Helper to set and clean up environment variables
struct EnvGuard {
    keys: Vec<String>,
}

impl EnvGuard {
    fn new() -> Self {
        EnvGuard { keys: Vec::new() }
    }

    fn set(&mut self, key: &str, value: &str) {
        env::set_var(key, value);
        self.keys.push(key.to_string());
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for key in &self.keys {
            env::remove_var(key);
        }
    }
}

#[test]
#[cfg(all(feature = "env", feature = "yaml"))]
fn test_precedence_env_over_yaml() {
    let mut env_guard = EnvGuard::new();

    // Create a temporary YAML file
    let mut yaml_file = NamedTempFile::new().unwrap();
    writeln!(yaml_file, "test.key: yaml_value").unwrap();

    // Set environment variable
    env_guard.set("TEST_KEY", "env_value");

    // Build service with both sources
    let service = DefaultConfigService::builder()
        .with_yaml_file(yaml_file.path())
        .unwrap()
        .with_env_vars()
        .build()
        .unwrap();

    // Environment variable should win (priority 2 > 1)
    let key = ConfigKey::from("test.key");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "env_value");
}

#[test]
#[cfg(all(feature = "cli", feature = "env"))]
fn test_precedence_cli_over_env() {
    let mut env_guard = EnvGuard::new();

    // Set environment variable
    env_guard.set("TEST_KEY", "env_value");

    // Build service with CLI args and env vars
    let args = vec!["--test.key", "cli_value"];
    let service = DefaultConfigService::builder()
        .with_env_vars()
        .with_cli_args(args)
        .build()
        .unwrap();

    // CLI should win (priority 3 > 2)
    let key = ConfigKey::from("test.key");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "cli_value");
}

#[test]
#[cfg(all(feature = "cli", feature = "yaml"))]
fn test_precedence_cli_over_yaml() {
    // Create a temporary YAML file
    let mut yaml_file = NamedTempFile::new().unwrap();
    writeln!(yaml_file, "test.key: yaml_value").unwrap();

    // Build service with CLI args and YAML
    let args = vec!["--test.key", "cli_value"];
    let service = DefaultConfigService::builder()
        .with_yaml_file(yaml_file.path())
        .unwrap()
        .with_cli_args(args)
        .build()
        .unwrap();

    // CLI should win (priority 3 > 1)
    let key = ConfigKey::from("test.key");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "cli_value");
}

#[test]
#[cfg(all(feature = "cli", feature = "env", feature = "yaml"))]
fn test_precedence_all_sources() {
    let mut env_guard = EnvGuard::new();

    // Create a temporary YAML file
    let mut yaml_file = NamedTempFile::new().unwrap();
    writeln!(
        yaml_file,
        "cli.key: yaml_value\nenv.key: yaml_value\nyaml.key: yaml_value"
    )
    .unwrap();

    // Set environment variables (only for cli.key and env.key, not yaml.key)
    env_guard.set("CLI_KEY", "env_value");
    env_guard.set("ENV_KEY", "env_value");

    // Build service with all sources
    let args = vec!["--cli.key", "cli_value"];
    let service = DefaultConfigService::builder()
        .with_yaml_file(yaml_file.path())
        .unwrap()
        .with_env_vars()
        .with_cli_args(args)
        .build()
        .unwrap();

    // Test CLI overrides everything
    let key = ConfigKey::from("cli.key");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "cli_value");

    // Test env overrides YAML
    let key = ConfigKey::from("env.key");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "env_value");

    // Test YAML is used when no higher priority source has the key
    let key = ConfigKey::from("yaml.key");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "yaml_value");
}

#[test]
#[cfg(feature = "yaml")]
fn test_yaml_only() {
    // Create a temporary YAML file
    let mut yaml_file = NamedTempFile::new().unwrap();
    writeln!(yaml_file, "test.key: yaml_value").unwrap();

    let service = DefaultConfigService::builder()
        .with_yaml_file(yaml_file.path())
        .unwrap()
        .build()
        .unwrap();

    let key = ConfigKey::from("test.key");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "yaml_value");
}

#[test]
#[cfg(feature = "env")]
fn test_env_only() {
    let mut env_guard = EnvGuard::new();
    env_guard.set("TEST_KEY", "env_value");

    let service = DefaultConfigService::builder()
        .with_env_vars()
        .build()
        .unwrap();

    let key = ConfigKey::from("test.key");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "env_value");
}

#[test]
#[cfg(feature = "cli")]
fn test_cli_only() {
    let args = vec!["--test.key", "cli_value"];
    let service = DefaultConfigService::builder()
        .with_cli_args(args)
        .build()
        .unwrap();

    let key = ConfigKey::from("test.key");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "cli_value");
}

#[test]
fn test_empty_service() {
    let service = DefaultConfigService::builder().build().unwrap();

    let key = ConfigKey::from("test.key");
    let result = service.get(&key);

    assert!(result.is_err());
}

#[test]
#[cfg(feature = "yaml")]
fn test_get_or_default_yaml() {
    // Create a temporary YAML file
    let mut yaml_file = NamedTempFile::new().unwrap();
    writeln!(yaml_file, "existing.key: yaml_value").unwrap();

    let service = DefaultConfigService::builder()
        .with_yaml_file(yaml_file.path())
        .unwrap()
        .build()
        .unwrap();

    // Test existing key
    let key = ConfigKey::from("existing.key");
    let value = service.get_or_default(&key, "default");
    assert_eq!(value.as_str(), "yaml_value");

    // Test nonexistent key
    let key = ConfigKey::from("nonexistent.key");
    let value = service.get_or_default(&key, "default");
    assert_eq!(value.as_str(), "default");
}

#[test]
#[cfg(feature = "yaml")]
fn test_has_key() {
    // Create a temporary YAML file
    let mut yaml_file = NamedTempFile::new().unwrap();
    writeln!(yaml_file, "existing.key: value").unwrap();

    let service = DefaultConfigService::builder()
        .with_yaml_file(yaml_file.path())
        .unwrap()
        .build()
        .unwrap();

    assert!(service.has(&ConfigKey::from("existing.key")));
    assert!(!service.has(&ConfigKey::from("nonexistent.key")));
}

#[test]
#[cfg(all(feature = "yaml", feature = "env"))]
fn test_partial_overlap() {
    let mut env_guard = EnvGuard::new();

    // Create a temporary YAML file with keys a, b, c
    let mut yaml_file = NamedTempFile::new().unwrap();
    writeln!(yaml_file, "key.a: yaml_a\nkey.b: yaml_b\nkey.c: yaml_c").unwrap();

    // Set environment variable only for key.a
    env_guard.set("KEY_A", "env_a");

    let service = DefaultConfigService::builder()
        .with_yaml_file(yaml_file.path())
        .unwrap()
        .with_env_vars()
        .build()
        .unwrap();

    // key.a should come from env (higher priority)
    let key = ConfigKey::from("key.a");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "env_a");

    // key.b and key.c should come from YAML
    let key = ConfigKey::from("key.b");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "yaml_b");

    let key = ConfigKey::from("key.c");
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "yaml_c");
}

#[test]
#[cfg(feature = "yaml")]
fn test_reload_service() {
    // Create a temporary YAML file
    let yaml_file = NamedTempFile::new().unwrap();
    let path = yaml_file.path().to_path_buf();

    std::fs::write(&path, "key: initial_value\n").unwrap();

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
    std::fs::write(&path, "key: updated_value\n").unwrap();

    // Reload service
    service.reload().unwrap();

    // Value should be updated
    let value = service.get(&key).unwrap();
    assert_eq!(value.as_str(), "updated_value");
}

#[test]
#[cfg(feature = "yaml")]
fn test_cache_behavior() {
    // Create a temporary YAML file
    let mut yaml_file = NamedTempFile::new().unwrap();
    writeln!(yaml_file, "key: value").unwrap();

    let service = DefaultConfigService::builder()
        .with_yaml_file(yaml_file.path())
        .unwrap()
        .build()
        .unwrap();

    let key = ConfigKey::from("key");

    // First access should populate cache
    let value1 = service.get(&key).unwrap();
    assert_eq!(value1.as_str(), "value");

    // Second access should use cache (same value)
    let value2 = service.get(&key).unwrap();
    assert_eq!(value2.as_str(), "value");

    // Values should be equal
    assert_eq!(value1.as_str(), value2.as_str());
}
