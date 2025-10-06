// SPDX-License-Identifier: MIT OR Apache-2.0

//! Command-line argument configuration source adapter.
//!
//! This module provides an adapter that reads configuration values from
//! command-line arguments.

use crate::domain::{ConfigKey, ConfigValue, Result};
use crate::ports::ConfigSource;
use std::collections::HashMap;

/// Configuration source adapter for command-line arguments.
///
/// This adapter reads configuration values from command-line arguments. It supports
/// multiple argument formats:
/// - `--key=value`: Long form with equals sign
/// - `--key value`: Long form with space-separated value
/// - `-k value`: Short form with space-separated value
///
/// # Priority
///
/// Command-line arguments have the highest priority (3), which means they override
/// both environment variables (priority 2) and configuration files (priority 1).
///
/// # Examples
///
/// ```rust
/// use configuration::adapters::CommandLineAdapter;
/// use configuration::ports::ConfigSource;
///
/// let args = vec!["--database.host=localhost", "--port", "5432"];
/// let adapter = CommandLineAdapter::from_args(args);
/// ```
#[derive(Debug, Clone)]
pub struct CommandLineAdapter {
    /// Parsed configuration values
    values: HashMap<String, String>,
}

impl CommandLineAdapter {
    /// Creates a new command-line adapter with no arguments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::adapters::CommandLineAdapter;
    ///
    /// let adapter = CommandLineAdapter::new();
    /// ```
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Creates a new command-line adapter from a vector of arguments.
    ///
    /// This method parses the arguments and extracts configuration key-value pairs.
    ///
    /// # Arguments
    ///
    /// * `args` - A vector of string arguments
    ///
    /// # Examples
    ///
    /// ```rust
    /// use configuration::adapters::CommandLineAdapter;
    ///
    /// let args = vec!["--database.host=localhost", "--port", "5432"];
    /// let adapter = CommandLineAdapter::from_args(args);
    /// ```
    pub fn from_args<S: AsRef<str>>(args: Vec<S>) -> Self {
        let mut adapter = Self::new();
        adapter.parse_args(args);
        adapter
    }

    /// Creates a new command-line adapter from the process's command-line arguments.
    ///
    /// This skips the first argument (the program name) and parses the rest.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::adapters::CommandLineAdapter;
    ///
    /// let adapter = CommandLineAdapter::from_env_args();
    /// ```
    pub fn from_env_args() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        Self::from_args(args)
    }

    /// Parses command-line arguments and populates the values map.
    fn parse_args<S: AsRef<str>>(&mut self, args: Vec<S>) {
        let mut i = 0;
        while i < args.len() {
            let arg = args[i].as_ref();

            // Handle --key=value format
            if arg.starts_with("--") && arg.contains('=') {
                if let Some((key, value)) = arg.strip_prefix("--").and_then(|s| s.split_once('=')) {
                    self.values.insert(key.to_string(), value.to_string());
                }
                i += 1;
            }
            // Handle --key value format
            else if arg.starts_with("--") {
                let key = arg.strip_prefix("--").unwrap();
                if i + 1 < args.len() {
                    let next_arg = args[i + 1].as_ref();
                    // Make sure the next argument is not another flag
                    if !next_arg.starts_with('-') {
                        self.values.insert(key.to_string(), next_arg.to_string());
                        i += 2;
                    } else {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
            // Handle -k value format (single character short form)
            else if arg.starts_with('-') && arg.len() == 2 {
                let key = arg.strip_prefix('-').unwrap();
                if i + 1 < args.len() {
                    let next_arg = args[i + 1].as_ref();
                    // Make sure the next argument is not another flag
                    if !next_arg.starts_with('-') {
                        self.values.insert(key.to_string(), next_arg.to_string());
                        i += 2;
                    } else {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }
}

impl Default for CommandLineAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for CommandLineAdapter {
    fn name(&self) -> &str {
        "cli"
    }

    fn priority(&self) -> u8 {
        3
    }

    fn get(&self, key: &ConfigKey) -> Result<Option<ConfigValue>> {
        Ok(self
            .values
            .get(key.as_str())
            .map(|v| ConfigValue::from(v.as_str())))
    }

    fn all_keys(&self) -> Result<Vec<ConfigKey>> {
        Ok(self
            .values
            .keys()
            .map(|k| ConfigKey::from(k.as_str()))
            .collect())
    }

    fn reload(&mut self) -> Result<()> {
        // Command-line arguments don't change during runtime
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_adapter_name() {
        let adapter = CommandLineAdapter::new();
        assert_eq!(adapter.name(), "cli");
    }

    #[test]
    fn test_cli_adapter_priority() {
        let adapter = CommandLineAdapter::new();
        assert_eq!(adapter.priority(), 3);
    }

    #[test]
    fn test_cli_adapter_empty() {
        let adapter = CommandLineAdapter::new();
        let key = ConfigKey::from("test.key");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_none());
    }

    #[test]
    fn test_cli_adapter_long_form_equals() {
        let args = vec!["--database.host=localhost", "--database.port=5432"];
        let adapter = CommandLineAdapter::from_args(args);

        let key = ConfigKey::from("database.host");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "localhost");

        let key = ConfigKey::from("database.port");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "5432");
    }

    #[test]
    fn test_cli_adapter_long_form_space() {
        let args = vec!["--host", "localhost", "--port", "8080"];
        let adapter = CommandLineAdapter::from_args(args);

        let key = ConfigKey::from("host");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "localhost");

        let key = ConfigKey::from("port");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "8080");
    }

    #[test]
    fn test_cli_adapter_short_form() {
        let args = vec!["-h", "localhost", "-p", "8080"];
        let adapter = CommandLineAdapter::from_args(args);

        let key = ConfigKey::from("h");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "localhost");

        let key = ConfigKey::from("p");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "8080");
    }

    #[test]
    fn test_cli_adapter_mixed_formats() {
        let args = vec!["--database.host=localhost", "--port", "5432", "-d", "mydb"];
        let adapter = CommandLineAdapter::from_args(args);

        let key = ConfigKey::from("database.host");
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "localhost");

        let key = ConfigKey::from("port");
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "5432");

        let key = ConfigKey::from("d");
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "mydb");
    }

    #[test]
    fn test_cli_adapter_missing_value() {
        let args = vec!["--host"];
        let adapter = CommandLineAdapter::from_args(args);

        let key = ConfigKey::from("host");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_none());
    }

    #[test]
    fn test_cli_adapter_flag_as_value() {
        // --host followed by another flag should not treat the flag as a value
        let args = vec!["--host", "--port", "8080"];
        let adapter = CommandLineAdapter::from_args(args);

        let key = ConfigKey::from("host");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_none());

        let key = ConfigKey::from("port");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "8080");
    }

    #[test]
    fn test_cli_adapter_equals_in_value() {
        let args = vec!["--connection-string=host=localhost;port=5432"];
        let adapter = CommandLineAdapter::from_args(args);

        let key = ConfigKey::from("connection-string");
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "host=localhost;port=5432");
    }

    #[test]
    fn test_cli_adapter_all_keys() {
        let args = vec!["--key1=value1", "--key2", "value2", "-k", "value3"];
        let adapter = CommandLineAdapter::from_args(args);

        let keys = adapter.all_keys().unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&ConfigKey::from("key1")));
        assert!(keys.contains(&ConfigKey::from("key2")));
        assert!(keys.contains(&ConfigKey::from("k")));
    }

    #[test]
    fn test_cli_adapter_reload() {
        let args = vec!["--test=value"];
        let mut adapter = CommandLineAdapter::from_args(args);

        // Reload should not fail (even though it's a no-op for CLI)
        assert!(adapter.reload().is_ok());
    }

    #[test]
    fn test_cli_adapter_default() {
        let adapter = CommandLineAdapter::default();
        assert_eq!(adapter.name(), "cli");
        assert_eq!(adapter.priority(), 3);
    }

    #[test]
    fn test_cli_adapter_empty_value() {
        let args = vec!["--key="];
        let adapter = CommandLineAdapter::from_args(args);

        let key = ConfigKey::from("key");
        let value = adapter.get(&key).unwrap();
        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "");
    }

    #[test]
    fn test_cli_adapter_non_flag_arguments() {
        // Non-flag arguments should be ignored
        let args = vec!["positional1", "--key", "value", "positional2"];
        let adapter = CommandLineAdapter::from_args(args);

        let keys = adapter.all_keys().unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&ConfigKey::from("key")));
    }

    #[test]
    fn test_cli_adapter_override_value() {
        // If the same key appears multiple times, the last value should win
        let args = vec!["--key=value1", "--key=value2"];
        let adapter = CommandLineAdapter::from_args(args);

        let key = ConfigKey::from("key");
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "value2");
    }
}
