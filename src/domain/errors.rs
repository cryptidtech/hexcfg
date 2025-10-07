// SPDX-License-Identifier: MIT OR Apache-2.0

//! Error types for the configuration crate.
//!
//! This module defines the error types that can occur when working with configuration sources.
//! All errors use `thiserror` for proper error handling and conversion.

use std::num::{ParseFloatError, ParseIntError};
use std::str::ParseBoolError;
use thiserror::Error;

/// The main error type for configuration operations.
///
/// This enum represents all possible errors that can occur when reading, parsing,
/// or accessing configuration values. It is marked as `#[non_exhaustive]` to allow
/// for future additions without breaking backwards compatibility.
///
/// # Examples
///
/// ```
/// use hexcfg::domain::errors::ConfigError;
///
/// fn get_config_value() -> Result<String, ConfigError> {
///     Err(ConfigError::ConfigKeyNotFound {
///         key: "database.host".to_string(),
///     })
/// }
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The requested configuration key was not found in any source.
    #[error("Configuration key not found: {key}")]
    ConfigKeyNotFound {
        /// The key that was not found
        key: String,
    },

    /// Failed to convert a configuration value to the requested type.
    #[error(
        "Failed to convert configuration value for key '{key}' to type {target_type}: {source}"
    )]
    TypeConversionError {
        /// The key being converted
        key: String,
        /// The target type name
        target_type: String,
        /// The underlying conversion error
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// An error occurred in a configuration source.
    #[error("Configuration source '{source_name}' error: {message}")]
    SourceError {
        /// The name of the source that encountered the error
        source_name: String,
        /// The error message
        message: String,
        /// The underlying error, if any
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Failed to parse a configuration file or value.
    #[error("Failed to parse configuration: {message}")]
    ParseError {
        /// The error message
        message: String,
        /// The underlying parsing error
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An error occurred in a configuration watcher.
    #[error("Configuration watcher error: {message}")]
    WatcherError {
        /// The error message
        message: String,
        /// The underlying error
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An I/O error occurred while reading configuration.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

// Implement conversions from common parsing errors to TypeConversionError
impl ConfigError {
    /// Creates a TypeConversionError from a ParseIntError.
    pub fn from_parse_int_error(key: String, err: ParseIntError) -> Self {
        ConfigError::TypeConversionError {
            key,
            target_type: "integer".to_string(),
            source: Box::new(err),
        }
    }

    /// Creates a TypeConversionError from a ParseFloatError.
    pub fn from_parse_float_error(key: String, err: ParseFloatError) -> Self {
        ConfigError::TypeConversionError {
            key,
            target_type: "float".to_string(),
            source: Box::new(err),
        }
    }

    /// Creates a TypeConversionError from a ParseBoolError.
    pub fn from_parse_bool_error(key: String, err: ParseBoolError) -> Self {
        ConfigError::TypeConversionError {
            key,
            target_type: "boolean".to_string(),
            source: Box::new(err),
        }
    }
}

/// A specialized Result type for configuration operations.
pub type Result<T> = std::result::Result<T, ConfigError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_key_not_found_error() {
        let error = ConfigError::ConfigKeyNotFound {
            key: "test.key".to_string(),
        };
        assert_eq!(error.to_string(), "Configuration key not found: test.key");
    }

    #[test]
    fn test_type_conversion_error() {
        let source_error = "invalid value".parse::<i32>().unwrap_err();
        let error = ConfigError::TypeConversionError {
            key: "test.key".to_string(),
            target_type: "i32".to_string(),
            source: Box::new(source_error),
        };
        assert!(error.to_string().contains("test.key"));
        assert!(error.to_string().contains("i32"));
    }

    #[test]
    fn test_source_error() {
        let error = ConfigError::SourceError {
            source_name: "env".to_string(),
            message: "Failed to read environment".to_string(),
            source: None,
        };
        assert_eq!(
            error.to_string(),
            "Configuration source 'env' error: Failed to read environment"
        );
    }

    #[test]
    fn test_parse_error() {
        let error = ConfigError::ParseError {
            message: "Invalid YAML".to_string(),
            source: None,
        };
        assert_eq!(
            error.to_string(),
            "Failed to parse configuration: Invalid YAML"
        );
    }

    #[test]
    fn test_watcher_error() {
        let error = ConfigError::WatcherError {
            message: "File watcher failed".to_string(),
            source: None,
        };
        assert_eq!(
            error.to_string(),
            "Configuration watcher error: File watcher failed"
        );
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error = ConfigError::from(io_error);
        assert!(matches!(error, ConfigError::IoError(_)));
    }

    #[test]
    fn test_from_parse_int_error() {
        let parse_err = "not_a_number".parse::<i32>().unwrap_err();
        let error = ConfigError::from_parse_int_error("test.key".to_string(), parse_err);
        assert!(matches!(error, ConfigError::TypeConversionError { .. }));
        assert!(error.to_string().contains("integer"));
    }

    #[test]
    fn test_from_parse_float_error() {
        let parse_err = "not_a_float".parse::<f64>().unwrap_err();
        let error = ConfigError::from_parse_float_error("test.key".to_string(), parse_err);
        assert!(matches!(error, ConfigError::TypeConversionError { .. }));
        assert!(error.to_string().contains("float"));
    }

    #[test]
    fn test_from_parse_bool_error() {
        let parse_err = "not_a_bool".parse::<bool>().unwrap_err();
        let error = ConfigError::from_parse_bool_error("test.key".to_string(), parse_err);
        assert!(matches!(error, ConfigError::TypeConversionError { .. }));
        assert!(error.to_string().contains("boolean"));
    }
}
