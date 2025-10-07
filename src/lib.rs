// SPDX-License-Identifier: MIT OR Apache-2.0

//! A hexagonal architecture configuration management crate.
//!
//! This crate provides a flexible, type-safe configuration management system that can
//! read configuration from multiple sources including environment variables, YAML files,
//! command-line arguments, and remote services like etcd and Redis.
//!
//! # Architecture
//!
//! The crate follows hexagonal architecture principles:
//!
//! - **Domain Layer**: Core types and business logic (`ConfigKey`, `ConfigValue`, errors)
//! - **Ports**: Trait definitions that define interfaces (`ConfigSource`, `ConfigWatcher`)
//! - **Adapters**: Implementations for specific configuration sources (env vars, YAML, etc.)
//! - **Service**: The main configuration service that orchestrates everything
//!
//! # Features
//!
//! - **Multiple Sources**: Environment variables, YAML files, CLI arguments, etcd, Redis
//! - **Type Safety**: Type-safe conversions from string values to Rust types
//! - **Precedence**: Configurable precedence order (CLI > env > files by default)
//! - **Dynamic Reloading**: Watch for configuration changes and reload automatically
//! - **Extensible**: Easy to add new configuration sources via trait implementation
//!
//! # Feature Flags
//!
//! - `yaml`: Enable YAML file support (default)
//! - `env`: Enable environment variable support (default)
//! - `cli`: Enable command-line argument support (default)
//! - `reload`: Enable dynamic reloading with file watching
//! - `etcd`: Enable etcd remote configuration support
//! - `redis`: Enable Redis remote configuration support
//! - `remote`: Enable all remote sources (etcd + redis)
//! - `full`: Enable all features
//!
//! # Quick Start
//!
//! ```rust
//! use configuration::prelude::*;
//!
//! # fn main() -> Result<()> {
//! // Create a configuration service with environment variables
//! let service = DefaultConfigService::builder()
//!     .with_env_vars()
//!     .build()?;
//!
//! // Get a configuration value (using convenient string slice methods)
//! let value = service.get_or_default_str("app.name", "MyApp");
//! println!("Application name: {}", value.as_str());
//!
//! // Type-safe conversions
//! if let Ok(port_value) = service.get_str("app.port") {
//!     let port: i32 = port_value.as_i32("app.port").unwrap_or(8080);
//!     println!("Port: {}", port);
//! }
//!
//! // Check if a key exists
//! if service.has_str("app.debug") {
//!     println!("Debug mode configured");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Convenience Methods
//!
//! For ergonomic usage, the crate provides `_str` variants that accept string slices:
//! - [`ConfigurationService::get_str`](domain::ConfigurationService::get_str)
//! - [`ConfigurationService::get_or_default_str`](domain::ConfigurationService::get_or_default_str)
//! - [`ConfigurationService::has_str`](domain::ConfigurationService::has_str)
//! - [`ConfigSource::get_str`](ports::ConfigSource::get_str)
//!
//! # Examples
//!
//! See the `examples/` directory for comprehensive examples:
//! - `basic_usage.rs` - Getting started with environment variables
//! - `multi_source.rs` - Using multiple configuration sources with precedence
//! - `dynamic_reload.rs` - Dynamic configuration reloading with file watching

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(clippy::all)]

pub mod adapters;
pub mod domain;
pub mod ports;
pub mod service;

/// Commonly used types and traits.
///
/// This module re-exports the most commonly used types and traits for convenient access.
pub mod prelude {
    pub use crate::domain::{ConfigError, ConfigKey, ConfigValue, ConfigurationService, Result};
    pub use crate::ports::{ConfigParser, ConfigSource, ConfigWatcher};
    pub use crate::service::{ConfigurationServiceBuilder, DefaultConfigService};

    // Re-export adapters based on feature flags
    #[cfg(feature = "cli")]
    pub use crate::adapters::CommandLineAdapter;
    #[cfg(feature = "env")]
    pub use crate::adapters::EnvVarAdapter;
    #[cfg(feature = "etcd")]
    pub use crate::adapters::EtcdAdapter;
    #[cfg(feature = "reload")]
    pub use crate::adapters::FileWatcher;
    #[cfg(feature = "redis")]
    pub use crate::adapters::{RedisAdapter, RedisStorageMode};
    #[cfg(feature = "yaml")]
    pub use crate::adapters::{YamlFileAdapter, YamlParser};
}
