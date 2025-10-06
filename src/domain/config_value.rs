// SPDX-License-Identifier: MIT OR Apache-2.0

//! Configuration value type with type-safe conversions.
//!
//! This module provides the `ConfigValue` type, which wraps configuration values
//! and provides type-safe conversion methods to various Rust types.

use crate::domain::errors::{ConfigError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A type-safe wrapper for configuration values.
///
/// `ConfigValue` stores configuration values as strings internally and provides
/// type-safe conversion methods to common Rust types. This allows configuration
/// sources to return a uniform type while still providing type safety at the
/// point of use.
///
/// # Examples
///
/// ```
/// use configuration::domain::config_value::ConfigValue;
///
/// let value = ConfigValue::new("42".to_string());
/// assert_eq!(value.as_str(), "42");
/// assert_eq!(value.as_i32("test.key").unwrap(), 42);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigValue(String);

impl ConfigValue {
    /// Creates a new `ConfigValue` from a `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use configuration::domain::config_value::ConfigValue;
    ///
    /// let value = ConfigValue::new("hello".to_string());
    /// assert_eq!(value.as_str(), "hello");
    /// ```
    pub fn new(value: String) -> Self {
        ConfigValue(value)
    }

    /// Returns the value as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use configuration::domain::config_value::ConfigValue;
    ///
    /// let value = ConfigValue::from("world");
    /// assert_eq!(value.as_str(), "world");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Converts the value into a `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use configuration::domain::config_value::ConfigValue;
    ///
    /// let value = ConfigValue::from("test");
    /// assert_eq!(value.as_string(), "test");
    /// ```
    pub fn as_string(&self) -> String {
        self.0.clone()
    }

    /// Converts the value to a boolean.
    ///
    /// Recognizes the following values (case-insensitive):
    /// - `true`: "true", "yes", "1", "on"
    /// - `false`: "false", "no", "0", "off"
    ///
    /// # Examples
    ///
    /// ```
    /// use configuration::domain::config_value::ConfigValue;
    ///
    /// let value = ConfigValue::from("true");
    /// assert_eq!(value.as_bool("test.key").unwrap(), true);
    ///
    /// let value = ConfigValue::from("yes");
    /// assert_eq!(value.as_bool("test.key").unwrap(), true);
    /// ```
    pub fn as_bool(&self, key: &str) -> Result<bool> {
        match self.0.to_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => Ok(true),
            "false" | "no" | "0" | "off" => Ok(false),
            _ => self
                .0
                .parse::<bool>()
                .map_err(|e| ConfigError::from_parse_bool_error(key.to_string(), e)),
        }
    }

    /// Converts the value to an `i32`.
    ///
    /// # Examples
    ///
    /// ```
    /// use configuration::domain::config_value::ConfigValue;
    ///
    /// let value = ConfigValue::from("42");
    /// assert_eq!(value.as_i32("test.key").unwrap(), 42);
    /// ```
    pub fn as_i32(&self, key: &str) -> Result<i32> {
        self.0
            .parse::<i32>()
            .map_err(|e| ConfigError::from_parse_int_error(key.to_string(), e))
    }

    /// Converts the value to an `i64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use configuration::domain::config_value::ConfigValue;
    ///
    /// let value = ConfigValue::from("9223372036854775807");
    /// assert_eq!(value.as_i64("test.key").unwrap(), 9223372036854775807);
    /// ```
    pub fn as_i64(&self, key: &str) -> Result<i64> {
        self.0
            .parse::<i64>()
            .map_err(|e| ConfigError::from_parse_int_error(key.to_string(), e))
    }

    /// Converts the value to a `u32`.
    ///
    /// # Examples
    ///
    /// ```
    /// use configuration::domain::config_value::ConfigValue;
    ///
    /// let value = ConfigValue::from("42");
    /// assert_eq!(value.as_u32("test.key").unwrap(), 42);
    /// ```
    pub fn as_u32(&self, key: &str) -> Result<u32> {
        self.0
            .parse::<u32>()
            .map_err(|e| ConfigError::from_parse_int_error(key.to_string(), e))
    }

    /// Converts the value to a `u64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use configuration::domain::config_value::ConfigValue;
    ///
    /// let value = ConfigValue::from("18446744073709551615");
    /// assert_eq!(value.as_u64("test.key").unwrap(), 18446744073709551615);
    /// ```
    pub fn as_u64(&self, key: &str) -> Result<u64> {
        self.0
            .parse::<u64>()
            .map_err(|e| ConfigError::from_parse_int_error(key.to_string(), e))
    }

    /// Converts the value to an `f64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use configuration::domain::config_value::ConfigValue;
    ///
    /// let value = ConfigValue::from("3.14");
    /// assert_eq!(value.as_f64("test.key").unwrap(), 3.14);
    /// ```
    pub fn as_f64(&self, key: &str) -> Result<f64> {
        self.0
            .parse::<f64>()
            .map_err(|e| ConfigError::from_parse_float_error(key.to_string(), e))
    }

    /// Parses the value into any type that implements `FromStr`.
    ///
    /// # Examples
    ///
    /// ```
    /// use configuration::domain::config_value::ConfigValue;
    /// use std::net::IpAddr;
    ///
    /// let value = ConfigValue::from("127.0.0.1");
    /// let ip: IpAddr = value.parse("test.key").unwrap();
    /// assert_eq!(ip.to_string(), "127.0.0.1");
    /// ```
    pub fn parse<T>(&self, key: &str) -> Result<T>
    where
        T: FromStr,
        T::Err: std::error::Error + Send + Sync + 'static,
    {
        self.0
            .parse::<T>()
            .map_err(|e| ConfigError::TypeConversionError {
                key: key.to_string(),
                target_type: std::any::type_name::<T>().to_string(),
                source: Box::new(e),
            })
    }
}

impl From<String> for ConfigValue {
    fn from(s: String) -> Self {
        ConfigValue(s)
    }
}

impl From<&str> for ConfigValue {
    fn from(s: &str) -> Self {
        ConfigValue(s.to_string())
    }
}

impl From<ConfigValue> for String {
    fn from(value: ConfigValue) -> Self {
        value.0
    }
}

impl AsRef<str> for ConfigValue {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn test_config_value_new() {
        let value = ConfigValue::new("test".to_string());
        assert_eq!(value.as_str(), "test");
    }

    #[test]
    fn test_config_value_from_string() {
        let value = ConfigValue::from("test".to_string());
        assert_eq!(value.as_str(), "test");
    }

    #[test]
    fn test_config_value_from_str() {
        let value = ConfigValue::from("test");
        assert_eq!(value.as_str(), "test");
    }

    #[test]
    fn test_config_value_as_string() {
        let value = ConfigValue::from("test");
        assert_eq!(value.as_string(), "test");
    }

    #[test]
    fn test_config_value_display() {
        let value = ConfigValue::from("test");
        assert_eq!(format!("{}", value), "test");
    }

    #[test]
    fn test_as_bool_true_variants() {
        let true_values = vec![
            "true", "True", "TRUE", "yes", "Yes", "YES", "1", "on", "On", "ON",
        ];
        for val in true_values {
            let value = ConfigValue::from(val);
            assert_eq!(
                value.as_bool("test.key").unwrap(),
                true,
                "Failed for value: {}",
                val
            );
        }
    }

    #[test]
    fn test_as_bool_false_variants() {
        let false_values = vec![
            "false", "False", "FALSE", "no", "No", "NO", "0", "off", "Off", "OFF",
        ];
        for val in false_values {
            let value = ConfigValue::from(val);
            assert_eq!(
                value.as_bool("test.key").unwrap(),
                false,
                "Failed for value: {}",
                val
            );
        }
    }

    #[test]
    fn test_as_bool_invalid() {
        let value = ConfigValue::from("invalid");
        assert!(value.as_bool("test.key").is_err());
    }

    #[test]
    fn test_as_i32() {
        let value = ConfigValue::from("42");
        assert_eq!(value.as_i32("test.key").unwrap(), 42);

        let value = ConfigValue::from("-42");
        assert_eq!(value.as_i32("test.key").unwrap(), -42);
    }

    #[test]
    fn test_as_i32_invalid() {
        let value = ConfigValue::from("not_a_number");
        assert!(value.as_i32("test.key").is_err());

        let value = ConfigValue::from("3.14");
        assert!(value.as_i32("test.key").is_err());
    }

    #[test]
    fn test_as_i64() {
        let value = ConfigValue::from("9223372036854775807");
        assert_eq!(value.as_i64("test.key").unwrap(), 9223372036854775807);

        let value = ConfigValue::from("-9223372036854775808");
        assert_eq!(value.as_i64("test.key").unwrap(), -9223372036854775808);
    }

    #[test]
    fn test_as_i64_invalid() {
        let value = ConfigValue::from("not_a_number");
        assert!(value.as_i64("test.key").is_err());
    }

    #[test]
    fn test_as_u32() {
        let value = ConfigValue::from("42");
        assert_eq!(value.as_u32("test.key").unwrap(), 42);

        let value = ConfigValue::from("4294967295");
        assert_eq!(value.as_u32("test.key").unwrap(), 4294967295);
    }

    #[test]
    fn test_as_u32_invalid() {
        let value = ConfigValue::from("-42");
        assert!(value.as_u32("test.key").is_err());
    }

    #[test]
    fn test_as_u64() {
        let value = ConfigValue::from("18446744073709551615");
        assert_eq!(value.as_u64("test.key").unwrap(), 18446744073709551615);
    }

    #[test]
    fn test_as_u64_invalid() {
        let value = ConfigValue::from("-42");
        assert!(value.as_u64("test.key").is_err());
    }

    #[test]
    fn test_as_f64() {
        let value = ConfigValue::from("3.14");
        assert_eq!(value.as_f64("test.key").unwrap(), 3.14);

        let value = ConfigValue::from("-3.14");
        assert_eq!(value.as_f64("test.key").unwrap(), -3.14);
    }

    #[test]
    fn test_as_f64_invalid() {
        let value = ConfigValue::from("not_a_number");
        assert!(value.as_f64("test.key").is_err());
    }

    #[test]
    fn test_parse_custom_type() {
        let value = ConfigValue::from("127.0.0.1");
        let ip: IpAddr = value.parse("test.key").unwrap();
        assert_eq!(ip.to_string(), "127.0.0.1");
    }

    #[test]
    fn test_parse_invalid() {
        let value = ConfigValue::from("not_an_ip");
        let result: Result<IpAddr> = value.parse("test.key");
        assert!(result.is_err());
    }

    #[test]
    fn test_clone() {
        let value1 = ConfigValue::from("test");
        let value2 = value1.clone();
        assert_eq!(value1, value2);
    }

    #[test]
    fn test_equality() {
        let value1 = ConfigValue::from("test");
        let value2 = ConfigValue::from("test");
        let value3 = ConfigValue::from("other");

        assert_eq!(value1, value2);
        assert_ne!(value1, value3);
    }

    #[test]
    fn test_as_ref() {
        let value = ConfigValue::from("test");
        let s: &str = value.as_ref();
        assert_eq!(s, "test");
    }

    #[test]
    fn test_string_from_config_value() {
        let value = ConfigValue::from("test");
        let s: String = value.into();
        assert_eq!(s, "test");
    }

    #[test]
    fn test_empty_string() {
        let value = ConfigValue::from("");
        assert_eq!(value.as_str(), "");
    }

    #[test]
    fn test_whitespace() {
        let value = ConfigValue::from("  spaces  ");
        assert_eq!(value.as_str(), "  spaces  ");
    }
}
