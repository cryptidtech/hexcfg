// SPDX-License-Identifier: MIT OR Apache-2.0

//! Multi-source configuration example.
//!
//! This example demonstrates:
//! - Using multiple configuration sources (YAML, environment variables, CLI args)
//! - Understanding priority order and precedence
//! - How higher priority sources override lower priority ones
//! - Using the builder pattern to compose configuration sources
//!
//! To run this example:
//! ```bash
//! # Create a sample config file
//! cat > /tmp/config.yaml <<EOF
//! app:
//!   name: "YamlApp"
//!   version: "1.0.0"
//!   port: 8080
//! database:
//!   host: "localhost"
//!   port: 5432
//! EOF
//!
//! # Set some environment variables (these override YAML)
//! export APP_NAME="EnvApp"
//! export DATABASE_HOST="db.example.com"
//!
//! # Run with CLI args (these override both YAML and env vars)
//! cargo run --example multi_source --features yaml,env,cli -- \
//!   --app.name=CliApp --app.port=9000
//! ```

use configuration::prelude::*;
use std::env;

fn main() -> Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("=== Configuration Crate: Multi-Source Example ===\n");

    // Create a temporary YAML config file for demonstration
    let yaml_content = r#"
app:
  name: "YamlApp"
  version: "1.0.0"
  port: 8080
  environment: "production"

database:
  host: "localhost"
  port: 5432
  username: "admin"

features:
  analytics: true
  debug_mode: false
"#;

    // Write to a temporary file
    let temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(temp_file.path(), yaml_content)?;

    println!("Created temporary YAML config file at: {:?}\n", temp_file.path());

    // Collect command-line arguments
    let cli_args: Vec<String> = env::args().collect();

    // Build a configuration service with multiple sources
    // Priority order (highest to lowest):
    //   1. CLI arguments (priority 3)
    //   2. Environment variables (priority 2)
    //   3. YAML file (priority 1)
    println!("Building configuration service with three sources:");
    println!("  1. CLI arguments (priority 3) - highest");
    println!("  2. Environment variables (priority 2)");
    println!("  3. YAML file (priority 1) - lowest\n");

    let service = ConfigurationServiceBuilder::new()
        .with_yaml_file(temp_file.path())?
        .with_env_vars()
        .with_cli_args(cli_args)
        .build()?;

    // Demonstrate precedence by checking various keys
    println!("=== Configuration Values (showing precedence) ===\n");

    // app.name - Could come from any source
    println!("--- app.name ---");
    print_value_source(&service, "app.name");
    println!("  YAML value: YamlApp");
    println!("  Env var: APP_NAME (if set)");
    println!("  CLI arg: --app.name=<value> (if provided)");

    // app.port - Could come from any source
    println!("\n--- app.port ---");
    print_value_source(&service, "app.port");
    println!("  YAML value: 8080");
    println!("  Env var: APP_PORT (if set)");
    println!("  CLI arg: --app.port=<value> (if provided)");

    // app.version - Only in YAML
    println!("\n--- app.version ---");
    print_value_source(&service, "app.version");
    println!("  YAML value: 1.0.0");
    println!("  Note: Not likely to be overridden");

    // database.host - Could come from any source
    println!("\n--- database.host ---");
    print_value_source(&service, "database.host");
    println!("  YAML value: localhost");
    println!("  Env var: DATABASE_HOST (if set)");
    println!("  CLI arg: --database.host=<value> (if provided)");

    // database.port - Could come from any source
    println!("\n--- database.port ---");
    print_value_source(&service, "database.port");
    println!("  YAML value: 5432");

    // features.analytics - Boolean value
    println!("\n--- features.analytics ---");
    print_bool_value(&service, "features.analytics");
    println!("  YAML value: true");

    // Show a summary
    println!("\n=== Precedence Summary ===");
    println!("When the same key exists in multiple sources:");
    println!("  • CLI arguments ALWAYS win (priority 3)");
    println!("  • Environment variables override YAML (priority 2)");
    println!("  • YAML file has the lowest priority (priority 1)");

    println!("\n=== Try It Yourself ===");
    println!("Run with different configurations:");
    println!("  1. Set env var:   export APP_NAME='MyEnvApp'");
    println!("  2. Override via CLI: cargo run --example multi_source -- --app.name=MyCliApp");
    println!("  3. CLI value will be used (highest priority)");

    Ok(())
}

/// Helper function to print a config value and indicate which source it came from
fn print_value_source(service: &DefaultConfigService, key: &str) {
    let config_key = ConfigKey::from(key);

    match service.get(&config_key) {
        Ok(value) => {
            println!("  Current value: {}", value.as_str());
            println!("  Source: (determined by priority order)");
        }
        Err(_) => {
            println!("  ✗ Not found in any source");
        }
    }
}

/// Helper function to print a boolean config value
fn print_bool_value(service: &DefaultConfigService, key: &str) {
    let config_key = ConfigKey::from(key);

    match service.get(&config_key) {
        Ok(value) => {
            match value.as_bool(key) {
                Ok(b) => println!("  Current value: {} (boolean)", b),
                Err(_) => println!("  Current value: {} (not a valid boolean)", value.as_str()),
            }
        }
        Err(_) => {
            println!("  ✗ Not found");
        }
    }
}
