// SPDX-License-Identifier: MIT OR Apache-2.0

//! Configuration key newtype for type-safe key handling.
//!
//! This module provides the `ConfigKey` type, which is a newtype wrapper around `String`
//! that provides type safety for configuration keys and prevents accidental string confusion.

use std::fmt;
use std::hash::{Hash, Hasher};

/// A type-safe wrapper for configuration keys.
///
/// `ConfigKey` is a newtype that wraps a `String` to provide type safety when working
/// with configuration keys. This prevents accidental mixing of configuration keys with
/// other string values and makes the API more self-documenting.
///
/// # Examples
///
/// ```
/// use hexcfg::domain::config_key::ConfigKey;
///
/// let key = ConfigKey::from("database.host");
/// let key2 = ConfigKey::from("database.port".to_string());
///
/// assert_eq!(key.as_str(), "database.host");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigKey(String);

impl ConfigKey {
    /// Creates a new `ConfigKey` from a `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use hexcfg::domain::config_key::ConfigKey;
    ///
    /// let key = ConfigKey::new("app.name".to_string());
    /// assert_eq!(key.as_str(), "app.name");
    /// ```
    pub fn new(key: String) -> Self {
        ConfigKey(key)
    }

    /// Returns the key as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use hexcfg::domain::config_key::ConfigKey;
    ///
    /// let key = ConfigKey::from("app.version");
    /// assert_eq!(key.as_str(), "app.version");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Converts the `ConfigKey` into its inner `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use hexcfg::domain::config_key::ConfigKey;
    ///
    /// let key = ConfigKey::from("app.debug");
    /// let inner = key.into_string();
    /// assert_eq!(inner, "app.debug");
    /// ```
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for ConfigKey {
    fn from(s: String) -> Self {
        ConfigKey(s)
    }
}

impl From<&str> for ConfigKey {
    fn from(s: &str) -> Self {
        ConfigKey(s.to_string())
    }
}

impl From<ConfigKey> for String {
    fn from(key: ConfigKey) -> Self {
        key.0
    }
}

impl AsRef<str> for ConfigKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConfigKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Hash for ConfigKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_config_key_new() {
        let key = ConfigKey::new("test.key".to_string());
        assert_eq!(key.as_str(), "test.key");
    }

    #[test]
    fn test_config_key_from_string() {
        let key = ConfigKey::from("test.key".to_string());
        assert_eq!(key.as_str(), "test.key");
    }

    #[test]
    fn test_config_key_from_str() {
        let key = ConfigKey::from("test.key");
        assert_eq!(key.as_str(), "test.key");
    }

    #[test]
    fn test_config_key_into_string() {
        let key = ConfigKey::from("test.key");
        let inner = key.into_string();
        assert_eq!(inner, "test.key");
    }

    #[test]
    fn test_config_key_display() {
        let key = ConfigKey::from("test.key");
        assert_eq!(format!("{}", key), "test.key");
    }

    #[test]
    fn test_config_key_debug() {
        let key = ConfigKey::from("test.key");
        assert_eq!(format!("{:?}", key), "ConfigKey(\"test.key\")");
    }

    #[test]
    fn test_config_key_equality() {
        let key1 = ConfigKey::from("test.key");
        let key2 = ConfigKey::from("test.key");
        let key3 = ConfigKey::from("other.key");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_config_key_clone() {
        let key1 = ConfigKey::from("test.key");
        let key2 = key1.clone();

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_config_key_hash() {
        let key1 = ConfigKey::from("test.key");
        let key2 = ConfigKey::from("test.key");
        let key3 = ConfigKey::from("other.key");

        let mut map = HashMap::new();
        map.insert(key1.clone(), "value1");

        assert_eq!(map.get(&key2), Some(&"value1"));
        assert_eq!(map.get(&key3), None);
    }

    #[test]
    fn test_config_key_as_ref() {
        let key = ConfigKey::from("test.key");
        let s: &str = key.as_ref();
        assert_eq!(s, "test.key");
    }

    #[test]
    fn test_string_from_config_key() {
        let key = ConfigKey::from("test.key");
        let s: String = key.into();
        assert_eq!(s, "test.key");
    }

    #[test]
    fn test_config_key_with_dots() {
        let key = ConfigKey::from("database.connection.host");
        assert_eq!(key.as_str(), "database.connection.host");
    }

    #[test]
    fn test_config_key_with_underscores() {
        let key = ConfigKey::from("app_name");
        assert_eq!(key.as_str(), "app_name");
    }

    #[test]
    fn test_config_key_empty() {
        let key = ConfigKey::from("");
        assert_eq!(key.as_str(), "");
    }
}
