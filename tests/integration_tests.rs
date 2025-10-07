// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for basic configuration service operations.
//!
//! These tests verify that the configuration service works correctly
//! with various sources and handles common use cases.

use configuration::adapters::{CommandLineAdapter, EnvVarAdapter, YamlFileAdapter};
use configuration::domain::{ConfigKey, ConfigurationService};
use configuration::service::{ConfigurationServiceBuilder, DefaultConfigService};
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_get_basic_value() {
    let mut env_vars = HashMap::new();
    env_vars.insert("test.key".to_string(), "test_value".to_string());

    let adapter = EnvVarAdapter::with_values(env_vars);

    let mut service = DefaultConfigService::new();
    service.add_source(Box::new(adapter));

    let key = ConfigKey::from("test.key");
    let value = service.get(&key).unwrap();

    assert_eq!(value.as_str(), "test_value");
}

#[test]
fn test_get_missing_key() {
    let service = DefaultConfigService::new();

    let key = ConfigKey::from("nonexistent.key");
    let result = service.get(&key);

    assert!(result.is_err());
}

#[test]
fn test_get_or_default() {
    let service = DefaultConfigService::new();

    let key = ConfigKey::from("missing.key");
    let value = service.get_or_default(&key, "default_value");

    assert_eq!(value.as_str(), "default_value");
}

#[test]
fn test_has_key() {
    let mut env_vars = HashMap::new();
    env_vars.insert("existing.key".to_string(), "value".to_string());

    let adapter = EnvVarAdapter::with_values(env_vars);
    let mut service = DefaultConfigService::new();
    service.add_source(Box::new(adapter));

    assert!(service.has(&ConfigKey::from("existing.key")));
    assert!(!service.has(&ConfigKey::from("missing.key")));
}

#[test]
fn test_type_conversions() {
    let mut env_vars = HashMap::new();
    env_vars.insert("bool_true".to_string(), "true".to_string());
    env_vars.insert("bool_false".to_string(), "false".to_string());
    env_vars.insert("int_value".to_string(), "42".to_string());
    env_vars.insert("float_value".to_string(), "3.14".to_string());
    env_vars.insert("string_value".to_string(), "hello".to_string());

    let adapter = EnvVarAdapter::with_values(env_vars);
    let mut service = DefaultConfigService::new();
    service.add_source(Box::new(adapter));

    // Test boolean conversions
    let bool_true = service.get(&ConfigKey::from("bool_true")).unwrap();
    assert_eq!(bool_true.as_bool("").unwrap(), true);

    let bool_false = service.get(&ConfigKey::from("bool_false")).unwrap();
    assert_eq!(bool_false.as_bool("").unwrap(), false);

    // Test integer conversions
    let int_val = service.get(&ConfigKey::from("int_value")).unwrap();
    assert_eq!(int_val.as_i32("").unwrap(), 42);
    assert_eq!(int_val.as_i64("").unwrap(), 42i64);
    assert_eq!(int_val.as_u32("").unwrap(), 42u32);
    assert_eq!(int_val.as_u64("").unwrap(), 42u64);

    // Test float conversion
    let float_val = service.get(&ConfigKey::from("float_value")).unwrap();
    assert!((float_val.as_f64("").unwrap() - 3.14).abs() < 0.001);

    // Test string conversion
    let string_val = service.get(&ConfigKey::from("string_value")).unwrap();
    assert_eq!(string_val.as_string(), "hello");
}

#[test]
fn test_invalid_type_conversions() {
    let mut env_vars = HashMap::new();
    env_vars.insert("invalid_int".to_string(), "not_a_number".to_string());
    env_vars.insert("invalid_bool".to_string(), "maybe".to_string());

    let adapter = EnvVarAdapter::with_values(env_vars);
    let mut service = DefaultConfigService::new();
    service.add_source(Box::new(adapter));

    // Test invalid integer conversion
    let val = service.get(&ConfigKey::from("invalid_int")).unwrap();
    assert!(val.as_i32("").is_err());

    // Test invalid boolean conversion
    let val = service.get(&ConfigKey::from("invalid_bool")).unwrap();
    assert!(val.as_bool("").is_err());
}

#[test]
fn test_yaml_file_source() {
    // Create a temporary YAML file
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "database:").unwrap();
    writeln!(temp_file, "  host: localhost").unwrap();
    writeln!(temp_file, "  port: 5432").unwrap();
    writeln!(temp_file, "app:").unwrap();
    writeln!(temp_file, "  name: TestApp").unwrap();
    temp_file.flush().unwrap();

    let adapter = YamlFileAdapter::from_file(temp_file.path()).unwrap();
    let mut service = DefaultConfigService::new();
    service.add_source(Box::new(adapter));

    // Test nested keys
    let host = service.get(&ConfigKey::from("database.host")).unwrap();
    assert_eq!(host.as_str(), "localhost");

    let port = service.get(&ConfigKey::from("database.port")).unwrap();
    assert_eq!(port.as_i32("").unwrap(), 5432);

    let app_name = service.get(&ConfigKey::from("app.name")).unwrap();
    assert_eq!(app_name.as_str(), "TestApp");
}

#[test]
fn test_cli_source() {
    let args = vec![
        "program".to_string(),
        "--config.key=value".to_string(),
        "--another".to_string(),
        "test".to_string(),
    ];

    let adapter = CommandLineAdapter::from_args(args);
    let mut service = DefaultConfigService::new();
    service.add_source(Box::new(adapter));

    let val1 = service.get(&ConfigKey::from("config.key")).unwrap();
    assert_eq!(val1.as_str(), "value");

    let val2 = service.get(&ConfigKey::from("another")).unwrap();
    assert_eq!(val2.as_str(), "test");
}

#[test]
fn test_multiple_sources() {
    // Create YAML file
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "key1: from_yaml").unwrap();
    writeln!(temp_file, "key2: yaml_value").unwrap();
    temp_file.flush().unwrap();

    // Create env vars
    let mut env_vars = HashMap::new();
    env_vars.insert("key1".to_string(), "from_env".to_string());
    env_vars.insert("key3".to_string(), "env_value".to_string());

    let yaml_adapter = YamlFileAdapter::from_file(temp_file.path()).unwrap();
    let env_adapter = EnvVarAdapter::with_values(env_vars);

    let mut service = DefaultConfigService::new();
    service.add_source(Box::new(yaml_adapter));
    service.add_source(Box::new(env_adapter));

    // key1 should come from env (higher priority)
    let val1 = service.get(&ConfigKey::from("key1")).unwrap();
    assert_eq!(val1.as_str(), "from_env");

    // key2 should come from yaml (only source)
    let val2 = service.get(&ConfigKey::from("key2")).unwrap();
    assert_eq!(val2.as_str(), "yaml_value");

    // key3 should come from env (only source)
    let val3 = service.get(&ConfigKey::from("key3")).unwrap();
    assert_eq!(val3.as_str(), "env_value");
}

#[test]
fn test_builder_pattern() {
    let mut env_vars = HashMap::new();
    env_vars.insert("ENV_KEY".to_string(), "env_value".to_string());
    let env_adapter = EnvVarAdapter::with_values(env_vars);

    let service = ConfigurationServiceBuilder::new()
        .with_source(Box::new(env_adapter))
        .build()
        .unwrap();

    assert!(service.has(&ConfigKey::from("ENV_KEY")));
}

#[test]
fn test_empty_service() {
    let service = DefaultConfigService::new();

    let key = ConfigKey::from("any.key");
    assert!(!service.has(&key));
    assert!(service.get(&key).is_err());
}

#[test]
fn test_cache_behavior() {
    let mut env_vars = HashMap::new();
    env_vars.insert("cached.key".to_string(), "value".to_string());

    let adapter = EnvVarAdapter::with_values(env_vars);
    let mut service = DefaultConfigService::new();
    service.add_source(Box::new(adapter));

    // First access
    let val1 = service.get(&ConfigKey::from("cached.key")).unwrap();
    assert_eq!(val1.as_str(), "value");

    // Second access (should use cache)
    let val2 = service.get(&ConfigKey::from("cached.key")).unwrap();
    assert_eq!(val2.as_str(), "value");
}

#[test]
fn test_special_characters_in_keys() {
    let mut env_vars = HashMap::new();
    env_vars.insert("key.with.dots".to_string(), "value1".to_string());
    env_vars.insert("key_with_underscores".to_string(), "value2".to_string());

    let adapter = EnvVarAdapter::with_values(env_vars);
    let mut service = DefaultConfigService::new();
    service.add_source(Box::new(adapter));

    let val1 = service.get(&ConfigKey::from("key.with.dots")).unwrap();
    assert_eq!(val1.as_str(), "value1");

    let val2 = service
        .get(&ConfigKey::from("key_with_underscores"))
        .unwrap();
    assert_eq!(val2.as_str(), "value2");
}

#[test]
fn test_empty_string_value() {
    let mut env_vars = HashMap::new();
    env_vars.insert("empty.key".to_string(), "".to_string());

    let adapter = EnvVarAdapter::with_values(env_vars);
    let mut service = DefaultConfigService::new();
    service.add_source(Box::new(adapter));

    let val = service.get(&ConfigKey::from("empty.key")).unwrap();
    assert_eq!(val.as_str(), "");
    assert!(val.as_bool("").is_err()); // Empty string is not a valid boolean
}

#[test]
fn test_whitespace_handling() {
    let mut env_vars = HashMap::new();
    env_vars.insert("whitespace.key".to_string(), "  value  ".to_string());

    let adapter = EnvVarAdapter::with_values(env_vars);
    let mut service = DefaultConfigService::new();
    service.add_source(Box::new(adapter));

    // Value should be preserved as-is (no automatic trimming)
    let val = service.get(&ConfigKey::from("whitespace.key")).unwrap();
    assert_eq!(val.as_str(), "  value  ");
}
