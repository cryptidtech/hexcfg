// SPDX-License-Identifier: MIT OR Apache-2.0

//! Domain layer containing core business logic and types.
//!
//! This module contains the core domain types and logic for the configuration crate.
//! It is independent of any external concerns and defines the fundamental concepts
//! used throughout the library.

pub mod config_key;
pub mod config_value;
pub mod errors;

// Re-export commonly used types
pub use config_key::ConfigKey;
pub use config_value::ConfigValue;
pub use errors::{ConfigError, Result};
