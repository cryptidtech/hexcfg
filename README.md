# Hexagonal Architecgture Configuration Service

[![Crates.io](https://img.shields.io/crates/v/configuration.svg)](https://crates.io/crates/configuration)
[![Documentation](https://docs.rs/configuration/badge.svg)](https://docs.rs/configuration)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A flexible, type-safe configuration management library for Rust applications, built with hexagonal architecture principles.

## Features

- **Multiple Configuration Sources**: Environment variables, YAML files, command-line arguments, etcd, and Redis
- **Type Safety**: Automatic type conversions with comprehensive error handling
- **Priority-Based Precedence**: CLI arguments override environment variables, which override configuration files
- **Dynamic Reloading**: Watch configuration files, etcd, and Redis for changes and reload automatically
- **Hexagonal Architecture**: Clean separation of concerns with domain, ports, and adapters
- **Extensible**: Easy to implement custom configuration sources via traits
- **Async Support**: Built-in support for async remote sources (etcd, Redis)

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
configuration = "0.7.0"
```

### Basic Usage

```rust
use configuration::prelude::*;

fn main() -> Result<()> {
    // Create a configuration service with environment variables
    let service = DefaultConfigService::builder()
        .with_env_vars()
        .build()?;

    // Get a configuration value (convenience method with string slice)
    let app_name = service.get_str("app.name")?;
    println!("Application name: {}", app_name.as_str());

    // Or use ConfigKey explicitly if preferred
    let app_name = service.get(&ConfigKey::from("app.name"))?;
    println!("Application name: {}", app_name.as_str());

    // Get with type conversion
    let port = service.get_str("app.port")?;
    let port_number: i32 = port.as_i32("app.port")?;

    // Use default values for optional configuration (convenience method)
    let log_level = service.get_or_default_str("log.level", "info");

    // Check if a key exists (convenience method)
    if service.has_str("app.debug") {
        println!("Debug mode is configured");
    }

    Ok(())
}
```

## Feature Flags

The crate uses feature flags to enable optional functionality:

| Feature | Description | Default |
|---------|-------------|---------|
| `yaml` | YAML file support via serde_yaml | ✅ |
| `env` | Environment variable support | ✅ |
| `cli` | Command-line argument support | ✅ |
| `reload` | Dynamic reloading with file watching | ❌ |
| `etcd` | etcd remote configuration support | ❌ |
| `redis` | Redis remote configuration support | ❌ |
| `remote` | All remote sources (etcd + redis) | ❌ |
| `full` | All features | ❌ |

### Custom Feature Configuration

```toml
[dependencies]
configuration = { version = "0.7.0", default-features = false, features = ["yaml", "env"] }
```

## Architecture

This crate follows hexagonal architecture principles:

```
┌───────────────────────────────────────────────────┐
│                     Application                   │
│                                                   │
│  ┌─────────────────────────────────────────────┐  │
│  │                 Domain Layer                │  │
│  │                                             │  │
│  │  • ConfigKey, ConfigValue (core types)      │  │
│  │  • ConfigurationService (business logic)    │  │
│  │  • ConfigError (error types)                │  │
│  │                                             │  │
│  └─────────────────────────────────────────────┘  │
│                         │                         │
│  ┌─────────────────────────────────────────────┐  │
│  │                 Ports Layer                 │  │
│  │                                             │  │
│  │  • ConfigSource trait (source interface)    │  │
│  │  • ConfigWatcher trait (watcher interface)  │  │
│  │  • ConfigParser trait (parser interface)    │  │
│  │                                             │  │
│  └─────────────────────────────────────────────┘  │
│                         │                         │
│  ┌─────────────────────────────────────────────┐  │
│  │                Adapters Layer               │  │
│  │                                             │  │
│  │  • YamlFileAdapter                          │  │
│  │  • EnvVarAdapter                            │  │
│  │  • CommandLineAdapter                       │  │
│  │  • EtcdAdapter                              │  │
│  │  • RedisAdapter                             │  │
│  │  • FileWatcher, EtcdWatcher, RedisWatcher   │  │
│  │                                             │  │
│  └─────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────┘
```

## String Convenience Methods

For more ergonomic usage, the crate provides `_str` variants of common methods that accept string slices directly:

```rust
use configuration::prelude::*;

fn main() -> Result<()> {
    let service = DefaultConfigService::builder()
        .with_env_vars()
        .build()?;

    // Use string slices directly without creating ConfigKey
    let value = service.get_str("app.name")?;              // Instead of get(&ConfigKey::from("app.name"))
    let value = service.get_or_default_str("log.level", "info");  // Instead of get_or_default(&ConfigKey::from(...), ...)
    let exists = service.has_str("app.debug");             // Instead of has(&ConfigKey::from("app.debug"))

    // Also available for ConfigSource trait
    let adapter = EnvVarAdapter::new();
    let value = adapter.get_str("database.host")?;        // Instead of get(&ConfigKey::from("database.host"))

    Ok(())
}
```

Both approaches work - use whichever feels more natural for your code style!

## Examples

### Multiple Configuration Sources

Combine multiple sources with automatic precedence handling:

```rust
use configuration::prelude::*;

fn main() -> Result<()> {
    let service = ConfigurationServiceBuilder::new()
        .with_yaml_file("/etc/myapp/config.yaml")?
        .with_env_vars()
        .with_cli_args(std::env::args().collect())
        .build()?;

    // CLI args (priority 3) override env vars (priority 2),
    // which override YAML files (priority 1)
    let value = service.get(&ConfigKey::from("database.host"))?;

    Ok(())
}
```

### Dynamic Configuration Reloading

Watch configuration files for changes:

```rust
use configuration::prelude::*;
use std::sync::{Arc, Mutex};

fn main() -> Result<()> {
    let service = Arc::new(Mutex::new(
        ConfigurationServiceBuilder::new()
            .with_yaml_file("/etc/myapp/config.yaml")?
            .build()?
    ));

    let mut watcher = FileWatcher::new(
        "/etc/myapp/config.yaml",
        None // Use default debounce delay
    )?;

    let service_clone = Arc::clone(&service);
    watcher.watch(Arc::new(move |_key| {
        println!("Configuration changed, reloading...");
        if let Ok(mut svc) = service_clone.lock() {
            let _ = svc.reload();
        }
    }))?;

    // Application continues running with live config updates

    Ok(())
}
```

### Type Conversions

Automatic type conversion with error handling:

```rust
use configuration::prelude::*;

fn main() -> Result<()> {
    let service = DefaultConfigService::builder()
        .with_env_vars()
        .build()?;

    // String value (no conversion)
    let name = service.get(&ConfigKey::from("app.name"))?;
    println!("Name: {}", name.as_str());

    // Integer conversion
    let port = service.get(&ConfigKey::from("app.port"))?;
    let port_i32: i32 = port.as_i32("app.port")?;
    let port_u16: u64 = port.as_u64("app.port")?;

    // Boolean conversion
    let debug = service.get(&ConfigKey::from("app.debug"))?;
    let debug_mode: bool = debug.as_bool("app.debug")?;

    // Float conversion
    let timeout = service.get(&ConfigKey::from("api.timeout"))?;
    let timeout_secs: f64 = timeout.as_f64("api.timeout")?;

    Ok(())
}
```

### Environment Variable Prefix Filtering

Filter environment variables by prefix:

```rust
use configuration::prelude::*;

fn main() -> Result<()> {
    // Only read environment variables starting with "MYAPP_"
    // MYAPP_DATABASE_HOST becomes "database.host"
    let service = ConfigurationServiceBuilder::new()
        .with_env_prefix("MYAPP_")
        .build()?;

    Ok(())
}
```

### Remote Configuration (etcd)

```rust
use configuration::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let service = ConfigurationServiceBuilder::new()
        .with_etcd(vec!["localhost:2379"], Some("myapp/")).await?
        .build()?;

    // Configuration is now loaded from etcd
    Ok(())
}
```

### Remote Configuration (Redis)

```rust
use configuration::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let service = ConfigurationServiceBuilder::new()
        .with_redis(
            "redis://localhost:6379",
            "myapp:",
            RedisStorageMode::StringKeys
        ).await?
        .build()?;

    // Configuration is now loaded from Redis
    Ok(())
}
```

### Watching Remote Configuration Changes

#### etcd Watcher

Watch for configuration changes in etcd using its native watch API:

```rust
use configuration::prelude::*;
use configuration::adapters::EtcdWatcher;
use configuration::ports::ConfigWatcher;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<()> {
    let service = Arc::new(Mutex::new(
        ConfigurationServiceBuilder::new()
            .with_etcd(vec!["localhost:2379"], Some("myapp/")).await?
            .build()?
    ));

    let mut watcher = EtcdWatcher::new(
        vec!["localhost:2379"],
        Some("myapp/")
    ).await?;

    let service_clone = Arc::clone(&service);
    watcher.watch(Arc::new(move |key| {
        println!("Configuration changed in etcd: {}", key);
        if let Ok(mut svc) = service_clone.lock() {
            let _ = svc.reload();
        }
    }))?;

    // Application continues running with live config updates from etcd

    Ok(())
}
```

#### Redis Watcher

Watch for configuration changes in Redis using keyspace notifications:

```rust
use configuration::prelude::*;
use configuration::adapters::RedisWatcher;
use configuration::ports::ConfigWatcher;
use std::sync::{Arc, Mutex};

fn main() -> Result<()> {
    let service = Arc::new(Mutex::new(
        ConfigurationServiceBuilder::new()
            .with_redis(
                "redis://localhost:6379",
                "myapp:",
                RedisStorageMode::StringKeys
            ).await?
            .build()?
    ));

    let mut watcher = RedisWatcher::new(
        "redis://localhost:6379",
        "myapp:"
    )?;

    // Try to enable keyspace notifications (requires CONFIG permission)
    let _ = watcher.try_enable_keyspace_notifications();

    let service_clone = Arc::clone(&service);
    watcher.watch(Arc::new(move |key| {
        println!("Configuration changed in Redis: {}", key);
        if let Ok(mut svc) = service_clone.lock() {
            let _ = svc.reload();
        }
    }))?;

    // Application continues running with live config updates from Redis

    Ok(())
}
```

**Note**: Redis keyspace notifications must be enabled on the Redis server:
```bash
# Via redis-cli
CONFIG SET notify-keyspace-events KEA

# Or in redis.conf
notify-keyspace-events KEA
```

## Custom Configuration Sources

Implement the `ConfigSource` trait to create custom sources:

```rust
use configuration::ports::ConfigSource;
use configuration::domain::{ConfigKey, ConfigValue, Result};

struct MyCustomSource;

impl ConfigSource for MyCustomSource {
    fn name(&self) -> &str {
        "my-custom-source"
    }

    fn priority(&self) -> u8 {
        1 // Lower than env vars but same as files
    }

    fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> {
        // Your custom logic here
        Ok(None)
    }

    fn all_keys(&self) -> Result<Vec<ConfigKey>> {
        Ok(vec![])
    }

    fn reload(&mut self) -> Result<()> {
        // Reload logic if applicable
        Ok(())
    }
}
```

## Priority System

Configuration sources have priorities that determine precedence:

| Priority | Source | Description |
|----------|--------|-------------|
| 3 | CLI Arguments | Highest priority, overrides all others |
| 2 | Environment Variables | Overrides files and remote sources |
| 1 | Files & Remote | YAML, etcd, Redis - lowest priority |

When multiple sources provide the same key, the value from the highest priority source is used.

## Error Handling

The crate provides comprehensive error types via `thiserror`:

```rust
use configuration::prelude::*;

fn load_config() -> Result<()> {
    let service = DefaultConfigService::builder()
        .with_yaml_file("/etc/myapp/config.yaml")?
        .build()?;

    match service.get(&ConfigKey::from("database.host")) {
        Ok(value) => println!("Host: {}", value.as_str()),
        Err(ConfigError::ConfigKeyNotFound { key }) => {
            eprintln!("Missing required configuration: {}", key);
        }
        Err(e) => eprintln!("Configuration error: {}", e),
    }

    Ok(())
}
```

## Running Examples

The crate includes several examples:

```bash
# Basic usage with environment variables
export APP_NAME="MyApp"
export APP_PORT="8080"
cargo run --example basic_usage --features env

# String convenience methods
export APP_NAME="MyApp"
export APP_PORT="8080"
cargo run --example string_convenience --features env

# Multiple sources with precedence
cargo run --example multi_source --features yaml,env,cli -- --app.name=CliApp

# Dynamic reloading
cargo run --example dynamic_reload --features yaml,reload
```

## Testing

Run tests with different feature combinations:

```bash
# Run all tests with default features
cargo test

# Run tests with all features
cargo test --all-features

# Run tests with specific features
cargo test --features yaml,env,cli

# Run property-based tests
cargo test --test proptest_tests
```

### Remote Watcher Integration Tests

Integration tests for etcd and Redis watchers are included in their respective integration test files and use Docker containers automatically via `testcontainers-rs`. These tests will automatically skip if Docker is not available:

```bash
# Run all Redis tests (including watcher tests)
cargo test --test redis_integration_tests --all-features

# Run all etcd tests (including watcher tests)
cargo test --test etcd_integration_tests --all-features

# Run specific watcher tests
cargo test --test redis_integration_tests test_redis_watcher --all-features
cargo test --test etcd_integration_tests test_etcd_watcher --all-features
```

Docker must be installed and running for these tests to execute. If Docker is unavailable, the tests will be skipped with a warning message.

## Documentation

Generate and view the full API documentation:

```bash
cargo doc --open --all-features
```

## Contributing

Contributions are welcome! Please ensure:

1. All tests pass: `cargo test --all-features`
2. Code is formatted: `cargo fmt --check`
3. No clippy warnings: `cargo clippy --all-features`
4. Documentation is updated for public APIs

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Project Status

This crate is currently at version 1.0.0 and includes:

- ✅ Core domain types (ConfigKey, ConfigValue)
- ✅ Configuration service with priority-based source management
- ✅ YAML file support
- ✅ Environment variable support
- ✅ Command-line argument support
- ✅ Dynamic reloading with file watching
- ✅ etcd integration
- ✅ Redis integration
- ✅ Comprehensive test suite (unit, integration, property-based)
- ✅ Full API documentation
- ✅ Example code

## Acknowledgments

This crate was designed with inspiration from configuration management libraries in other ecosystems, adapted to Rust's ownership model and type system.
