// SPDX-License-Identifier: MIT OR Apache-2.0

//! YAML file configuration source adapter.
//!
//! This module provides an adapter that reads configuration values from YAML files.

use crate::domain::{ConfigError, ConfigKey, ConfigValue, Result};
use crate::ports::{ConfigParser, ConfigSource};
use directories::ProjectDirs;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum allowed file size for YAML configuration files (10MB)
/// This prevents denial of service attacks via extremely large files
const MAX_YAML_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// YAML parser implementation.
///
/// This parser converts YAML files into flat key-value maps using dot notation
/// for nested structures.
///
/// # Examples
///
/// ```rust
/// use configuration::adapters::YamlParser;
/// use configuration::ports::ConfigParser;
///
/// let parser = YamlParser::new();
/// let yaml_content = "database:\n  host: localhost\n  port: 5432";
/// let result = parser.parse(yaml_content).unwrap();
/// assert_eq!(result.get("database.host"), Some(&"localhost".to_string()));
/// ```
#[derive(Debug, Clone)]
pub struct YamlParser;

impl YamlParser {
    /// Creates a new YAML parser.
    pub fn new() -> Self {
        YamlParser
    }

    /// Flattens a YAML value into a flat map with dot notation keys.
    fn flatten_yaml(value: &serde_yaml::Value, prefix: &str, result: &mut HashMap<String, String>) {
        match value {
            serde_yaml::Value::Mapping(map) => {
                for (key, val) in map {
                    if let Some(key_str) = key.as_str() {
                        let new_prefix = if prefix.is_empty() {
                            key_str.to_string()
                        } else {
                            format!("{}.{}", prefix, key_str)
                        };
                        Self::flatten_yaml(val, &new_prefix, result);
                    }
                }
            }
            serde_yaml::Value::Sequence(seq) => {
                for (i, val) in seq.iter().enumerate() {
                    let new_prefix = format!("{}.{}", prefix, i);
                    Self::flatten_yaml(val, &new_prefix, result);
                }
            }
            serde_yaml::Value::String(s) => {
                result.insert(prefix.to_string(), s.clone());
            }
            serde_yaml::Value::Number(n) => {
                result.insert(prefix.to_string(), n.to_string());
            }
            serde_yaml::Value::Bool(b) => {
                result.insert(prefix.to_string(), b.to_string());
            }
            serde_yaml::Value::Null => {
                result.insert(prefix.to_string(), String::new());
            }
            _ => {}
        }
    }
}

impl Default for YamlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigParser for YamlParser {
    fn parse(&self, content: &str) -> Result<HashMap<String, String>> {
        let value: serde_yaml::Value =
            serde_yaml::from_str(content).map_err(|e| ConfigError::ParseError {
                message: format!("Failed to parse YAML: {}", e),
                source: Some(Box::new(e)),
            })?;

        let mut result = HashMap::new();
        Self::flatten_yaml(&value, "", &mut result);
        Ok(result)
    }

    fn supported_extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

/// Configuration source adapter for YAML files.
///
/// This adapter reads configuration values from YAML files. It supports automatic
/// discovery of configuration files in OS-appropriate locations, as well as custom
/// file paths.
///
/// # Priority
///
/// YAML files have a priority of 1, which means they are overridden by both
/// environment variables (priority 2) and command-line arguments (priority 3).
///
/// # Examples
///
/// ```rust,no_run
/// use configuration::adapters::YamlFileAdapter;
/// use configuration::ports::ConfigSource;
///
/// // Load from a specific file
/// let adapter = YamlFileAdapter::from_file("/path/to/config.yaml").unwrap();
///
/// // Load from default OS location
/// let adapter = YamlFileAdapter::from_default_location("myapp", "com.example").unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct YamlFileAdapter {
    /// Path to the YAML file
    file_path: PathBuf,
    /// Parsed configuration values
    values: HashMap<String, String>,
    /// YAML parser
    parser: YamlParser,
}

impl YamlFileAdapter {
    /// Creates a new YAML file adapter from a specific file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the YAML file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::adapters::YamlFileAdapter;
    ///
    /// let adapter = YamlFileAdapter::from_file("/etc/myapp/config.yaml").unwrap();
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file_path = path.as_ref().to_path_buf();
        let parser = YamlParser::new();

        // Canonicalize path to prevent directory traversal attacks
        let canonical_path = file_path.canonicalize().map_err(|e| ConfigError::SourceError {
            source_name: "yaml-file".to_string(),
            message: format!(
                "Invalid or inaccessible path: {}",
                file_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<unknown>")
            ),
            source: Some(Box::new(e)),
        })?;

        // Check file size before reading to prevent DoS via large files
        let metadata = fs::metadata(&canonical_path).map_err(|e| ConfigError::SourceError {
            source_name: "yaml-file".to_string(),
            message: format!(
                "Failed to read file metadata: {}",
                canonical_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<unknown>")
            ),
            source: Some(Box::new(e)),
        })?;

        if metadata.len() > MAX_YAML_FILE_SIZE {
            return Err(ConfigError::SourceError {
                source_name: "yaml-file".to_string(),
                message: format!(
                    "Configuration file too large: {} bytes (max {} bytes)",
                    metadata.len(),
                    MAX_YAML_FILE_SIZE
                ),
                source: None,
            });
        }

        // Read file content
        let content = fs::read_to_string(&canonical_path).map_err(|e| ConfigError::SourceError {
            source_name: "yaml-file".to_string(),
            message: format!(
                "Failed to read configuration file: {}",
                canonical_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<unknown>")
            ),
            source: Some(Box::new(e)),
        })?;

        let values = parser.parse(&content)?;

        Ok(Self {
            file_path: canonical_path,
            values,
            parser,
        })
    }

    /// Creates a new YAML file adapter from the default OS-appropriate location.
    ///
    /// This method uses the `directories` crate to determine the appropriate
    /// configuration directory for the current operating system.
    ///
    /// # Arguments
    ///
    /// * `app_name` - The application name (e.g., "myapp")
    /// * `qualifier` - The organization/qualifier (e.g., "com.example")
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::adapters::YamlFileAdapter;
    ///
    /// let adapter = YamlFileAdapter::from_default_location("myapp", "com.example").unwrap();
    /// ```
    pub fn from_default_location(app_name: &str, qualifier: &str) -> Result<Self> {
        let proj_dirs =
            ProjectDirs::from(qualifier, "", app_name).ok_or_else(|| ConfigError::SourceError {
                source_name: "yaml-file".to_string(),
                message: "Failed to determine project directories".to_string(),
                source: None,
            })?;

        let config_dir = proj_dirs.config_dir();
        let config_file = config_dir.join("config.yaml");

        Self::from_file(config_file)
    }

    /// Creates a new YAML file adapter with a custom file name in the default location.
    ///
    /// # Arguments
    ///
    /// * `app_name` - The application name
    /// * `qualifier` - The organization/qualifier
    /// * `filename` - The configuration file name (e.g., "settings.yaml")
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use configuration::adapters::YamlFileAdapter;
    ///
    /// let adapter = YamlFileAdapter::with_filename("myapp", "com.example", "settings.yaml").unwrap();
    /// ```
    pub fn with_filename(app_name: &str, qualifier: &str, filename: &str) -> Result<Self> {
        let proj_dirs =
            ProjectDirs::from(qualifier, "", app_name).ok_or_else(|| ConfigError::SourceError {
                source_name: "yaml-file".to_string(),
                message: "Failed to determine project directories".to_string(),
                source: None,
            })?;

        let config_dir = proj_dirs.config_dir();
        let config_file = config_dir.join(filename);

        Self::from_file(config_file)
    }

    /// Returns the path to the configuration file.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }
}

impl ConfigSource for YamlFileAdapter {
    fn name(&self) -> &str {
        "yaml-file"
    }

    fn priority(&self) -> u8 {
        1
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
        // Check file size before reading
        let metadata = fs::metadata(&self.file_path).map_err(|e| ConfigError::SourceError {
            source_name: "yaml-file".to_string(),
            message: format!(
                "Failed to read file metadata: {}",
                self.file_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<unknown>")
            ),
            source: Some(Box::new(e)),
        })?;

        if metadata.len() > MAX_YAML_FILE_SIZE {
            return Err(ConfigError::SourceError {
                source_name: "yaml-file".to_string(),
                message: format!(
                    "Configuration file too large: {} bytes (max {} bytes)",
                    metadata.len(),
                    MAX_YAML_FILE_SIZE
                ),
                source: None,
            });
        }

        let content =
            fs::read_to_string(&self.file_path).map_err(|e| ConfigError::SourceError {
                source_name: "yaml-file".to_string(),
                message: format!(
                    "Failed to read configuration file: {}",
                    self.file_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("<unknown>")
                ),
                source: Some(Box::new(e)),
            })?;

        self.values = self.parser.parse(&content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_yaml_parser_simple() {
        let parser = YamlParser::new();
        let yaml = "key: value";
        let result = parser.parse(yaml).unwrap();

        assert_eq!(result.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_yaml_parser_nested() {
        let parser = YamlParser::new();
        let yaml = r#"
database:
  host: localhost
  port: 5432
"#;
        let result = parser.parse(yaml).unwrap();

        assert_eq!(result.get("database.host"), Some(&"localhost".to_string()));
        assert_eq!(result.get("database.port"), Some(&"5432".to_string()));
    }

    #[test]
    fn test_yaml_parser_deeply_nested() {
        let parser = YamlParser::new();
        let yaml = r#"
app:
  database:
    connection:
      host: localhost
      port: 5432
"#;
        let result = parser.parse(yaml).unwrap();

        assert_eq!(
            result.get("app.database.connection.host"),
            Some(&"localhost".to_string())
        );
        assert_eq!(
            result.get("app.database.connection.port"),
            Some(&"5432".to_string())
        );
    }

    #[test]
    fn test_yaml_parser_array() {
        let parser = YamlParser::new();
        let yaml = r#"
servers:
  - server1
  - server2
  - server3
"#;
        let result = parser.parse(yaml).unwrap();

        assert_eq!(result.get("servers.0"), Some(&"server1".to_string()));
        assert_eq!(result.get("servers.1"), Some(&"server2".to_string()));
        assert_eq!(result.get("servers.2"), Some(&"server3".to_string()));
    }

    #[test]
    fn test_yaml_parser_mixed_types() {
        let parser = YamlParser::new();
        let yaml = r#"
string_value: hello
number_value: 42
bool_value: true
null_value: null
"#;
        let result = parser.parse(yaml).unwrap();

        assert_eq!(result.get("string_value"), Some(&"hello".to_string()));
        assert_eq!(result.get("number_value"), Some(&"42".to_string()));
        assert_eq!(result.get("bool_value"), Some(&"true".to_string()));
        assert_eq!(result.get("null_value"), Some(&"".to_string()));
    }

    #[test]
    fn test_yaml_parser_invalid() {
        let parser = YamlParser::new();
        let yaml = "invalid: yaml: content:";
        let result = parser.parse(yaml);

        assert!(result.is_err());
    }

    #[test]
    fn test_yaml_parser_supported_extensions() {
        let parser = YamlParser::new();
        let extensions = parser.supported_extensions();

        assert_eq!(extensions.len(), 2);
        assert!(extensions.contains(&"yaml"));
        assert!(extensions.contains(&"yml"));
    }

    #[test]
    fn test_yaml_adapter_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "database:\n  host: localhost\n  port: 5432").unwrap();

        let adapter = YamlFileAdapter::from_file(temp_file.path()).unwrap();

        assert_eq!(adapter.name(), "yaml-file");
        assert_eq!(adapter.priority(), 1);

        let key = ConfigKey::from("database.host");
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "localhost");
    }

    #[test]
    fn test_yaml_adapter_all_keys() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "key1: value1\nkey2: value2").unwrap();

        let adapter = YamlFileAdapter::from_file(temp_file.path()).unwrap();
        let keys = adapter.all_keys().unwrap();

        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&ConfigKey::from("key1")));
        assert!(keys.contains(&ConfigKey::from("key2")));
    }

    #[test]
    fn test_yaml_adapter_reload() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Write initial content
        fs::write(&path, "key: initial_value\n").unwrap();

        let mut adapter = YamlFileAdapter::from_file(&path).unwrap();

        let key = ConfigKey::from("key");
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "initial_value");

        // Update file
        fs::write(&path, "key: updated_value\n").unwrap();

        // Reload
        adapter.reload().unwrap();

        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "updated_value");
    }

    #[test]
    fn test_yaml_adapter_nonexistent_key() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "key: value").unwrap();

        let adapter = YamlFileAdapter::from_file(temp_file.path()).unwrap();
        let key = ConfigKey::from("nonexistent");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_none());
    }

    #[test]
    fn test_yaml_adapter_file_path() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "key: value").unwrap();

        let adapter = YamlFileAdapter::from_file(temp_file.path()).unwrap();
        assert_eq!(adapter.file_path(), temp_file.path());
    }

    #[test]
    fn test_yaml_adapter_nonexistent_file() {
        let result = YamlFileAdapter::from_file("/nonexistent/path/to/config.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_yaml_parser_default() {
        let parser = YamlParser::default();
        assert_eq!(parser.supported_extensions().len(), 2);
    }
}
