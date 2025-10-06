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
//! ```rust,no_run
//! use configuration::prelude::*;
//!
//! # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//! // Configuration service will be available in later phases
//! # Ok(())
//! # }
//! ```
//!
//! # Examples
//!
//! More detailed examples will be added as the implementation progresses.

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(clippy::all)]

pub mod adapters;
pub mod domain;
pub mod ports;

/// Commonly used types and traits.
///
/// This module re-exports the most commonly used types and traits for convenient access.
pub mod prelude {
    pub use crate::domain::{ConfigError, ConfigKey, ConfigValue, ConfigurationService, Result};
    pub use crate::ports::{ConfigParser, ConfigSource, ConfigWatcher};

    // Re-export adapters based on feature flags
    #[cfg(feature = "cli")]
    pub use crate::adapters::CommandLineAdapter;
    #[cfg(feature = "env")]
    pub use crate::adapters::EnvVarAdapter;
    #[cfg(feature = "yaml")]
    pub use crate::adapters::{YamlFileAdapter, YamlParser};
}
