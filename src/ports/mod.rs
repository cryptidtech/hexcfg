// SPDX-License-Identifier: MIT OR Apache-2.0

//! Ports layer containing trait definitions.
//!
//! This module contains the trait definitions (ports) that define the interfaces
//! for various components of the configuration system. These traits are implemented
//! by adapters in the adapters layer.

pub mod parser;
pub mod source;
pub mod watcher;

// Re-export commonly used types
pub use parser::ConfigParser;
pub use source::ConfigSource;
pub use watcher::{ChangeCallback, ConfigWatcher};
