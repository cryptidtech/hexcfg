// SPDX-License-Identifier: MIT OR Apache-2.0

//! Configuration parser trait definition.
//!
//! This module defines the `ConfigParser` trait, which provides an interface for
//! parsing configuration files in different formats (YAML, TOML, JSON, etc.).

use crate::domain::Result;
use std::collections::HashMap;

/// A trait for parsing configuration files.
///
/// This trait defines the interface for implementing parsers that can read
/// configuration data from various file formats and convert it into a flat
/// key-value structure.
///
/// # Key Format
///
/// Parsers should flatten nested structures using dot notation. For example,
/// a YAML structure like:
///
/// ```yaml
/// database:
///   host: localhost
///   port: 5432
/// ```
///
/// Should be parsed into:
/// - `database.host` -> `"localhost"`
/// - `database.port` -> `"5432"`
///
/// # Examples
///
/// ```rust
/// use hexcfg::ports::ConfigParser;
/// use hexcfg::domain::Result;
/// use std::collections::HashMap;
///
/// struct MyParser;
///
/// impl ConfigParser for MyParser {
///     fn parse(&self, content: &str) -> Result<HashMap<String, String>> {
///         // Implementation here
///         Ok(HashMap::new())
///     }
///
///     fn supported_extensions(&self) -> &[&str] {
///         &["myformat"]
///     }
/// }
/// ```
pub trait ConfigParser {
    /// Parses configuration content into a flat key-value map.
    ///
    /// This method takes the raw content of a configuration file and parses it
    /// into a flat HashMap where keys use dot notation for nested structures.
    ///
    /// # Arguments
    ///
    /// * `content` - The raw content of the configuration file
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<String, String>)` - The parsed configuration as key-value pairs
    /// * `Err(ConfigError)` - An error occurred during parsing
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hexcfg::ports::ConfigParser;
    /// # use hexcfg::domain::Result;
    /// # use std::collections::HashMap;
    /// # struct MyParser;
    /// # impl ConfigParser for MyParser {
    /// #     fn parse(&self, content: &str) -> Result<HashMap<String, String>> {
    /// #         let mut map = HashMap::new();
    /// #         map.insert("key".to_string(), "value".to_string());
    /// #         Ok(map)
    /// #     }
    /// #     fn supported_extensions(&self) -> &[&str] { &["txt"] }
    /// # }
    /// let parser = MyParser;
    /// let content = "key: value";
    /// let result = parser.parse(content).unwrap();
    /// assert_eq!(result.get("key"), Some(&"value".to_string()));
    /// ```
    fn parse(&self, content: &str) -> Result<HashMap<String, String>>;

    /// Returns the file extensions supported by this parser.
    ///
    /// This allows the configuration system to automatically select the appropriate
    /// parser based on the file extension.
    ///
    /// # Returns
    ///
    /// A slice of file extensions (without the leading dot) that this parser supports.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hexcfg::ports::ConfigParser;
    /// # use hexcfg::domain::Result;
    /// # use std::collections::HashMap;
    /// # struct YamlParser;
    /// # impl ConfigParser for YamlParser {
    /// #     fn parse(&self, content: &str) -> Result<HashMap<String, String>> {
    /// #         Ok(HashMap::new())
    /// #     }
    /// #     fn supported_extensions(&self) -> &[&str] {
    /// #         &["yaml", "yml"]
    /// #     }
    /// # }
    /// let parser = YamlParser;
    /// let extensions = parser.supported_extensions();
    /// assert_eq!(extensions, &["yaml", "yml"]);
    /// ```
    fn supported_extensions(&self) -> &[&str];
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test implementation of ConfigParser for testing purposes
    struct TestParser;

    impl ConfigParser for TestParser {
        fn parse(&self, _content: &str) -> Result<HashMap<String, String>> {
            let mut map = HashMap::new();
            map.insert("test.key".to_string(), "test.value".to_string());
            Ok(map)
        }

        fn supported_extensions(&self) -> &[&str] {
            &["test", "tst"]
        }
    }

    #[test]
    fn test_parser_parse() {
        let parser = TestParser;
        let result = parser.parse("dummy content").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result.get("test.key"), Some(&"test.value".to_string()));
    }

    #[test]
    fn test_parser_supported_extensions() {
        let parser = TestParser;
        let extensions = parser.supported_extensions();
        assert_eq!(extensions.len(), 2);
        assert_eq!(extensions[0], "test");
        assert_eq!(extensions[1], "tst");
    }

    #[test]
    fn test_parser_parse_empty_content() {
        let parser = TestParser;
        let result = parser.parse("").unwrap();
        // Our test parser always returns one key
        assert_eq!(result.len(), 1);
    }

    // Test parser that flattens nested structures
    struct FlatteningParser;

    impl ConfigParser for FlatteningParser {
        fn parse(&self, _content: &str) -> Result<HashMap<String, String>> {
            let mut map = HashMap::new();
            map.insert("database.host".to_string(), "localhost".to_string());
            map.insert("database.port".to_string(), "5432".to_string());
            map.insert("app.name".to_string(), "MyApp".to_string());
            Ok(map)
        }

        fn supported_extensions(&self) -> &[&str] {
            &["flat"]
        }
    }

    #[test]
    fn test_flattening_parser() {
        let parser = FlatteningParser;
        let result = parser.parse("").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result.get("database.host"), Some(&"localhost".to_string()));
        assert_eq!(result.get("database.port"), Some(&"5432".to_string()));
        assert_eq!(result.get("app.name"), Some(&"MyApp".to_string()));
    }
}
