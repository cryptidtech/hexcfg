// SPDX-License-Identifier: MIT OR Apache-2.0

//! Service layer containing the configuration service implementations.
//!
//! This module contains the concrete implementations of the `ConfigurationService`
//! trait, which provides the main interface for accessing configuration values.

pub mod default_service;

// Re-export commonly used types
pub use default_service::{ConfigurationServiceBuilder, DefaultConfigService};
