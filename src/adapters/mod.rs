// SPDX-License-Identifier: MIT OR Apache-2.0

//! Adapters layer containing configuration source implementations.
//!
//! This module contains concrete implementations of the configuration source
//! traits defined in the ports layer. Each adapter implements the `ConfigSource`
//! trait to provide configuration from a specific source.

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "env")]
pub mod env_var;
#[cfg(feature = "etcd")]
pub mod etcd;
#[cfg(feature = "redis_backend")]
pub mod redis;
#[cfg(feature = "yaml")]
pub mod yaml_file;

// Re-export adapters based on feature flags
#[cfg(feature = "cli")]
pub use cli::CommandLineAdapter;
#[cfg(feature = "env")]
pub use env_var::EnvVarAdapter;
#[cfg(feature = "etcd")]
pub use etcd::EtcdAdapter;
#[cfg(feature = "redis_backend")]
pub use redis::{RedisAdapter, RedisStorageMode};
#[cfg(feature = "yaml")]
pub use yaml_file::{YamlFileAdapter, YamlParser};
