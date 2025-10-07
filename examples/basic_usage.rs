// SPDX-License-Identifier: MIT OR Apache-2.0

//! Basic usage example for the configuration crate.
//!
//! This example demonstrates:
//! - Creating a configuration service with environment variables
//! - Retrieving configuration values
//! - Type conversions (string, int, bool, float)
//! - Using default values for missing keys
//!
//! To run this example:
//! ```bash
//! # Set some environment variables
//! export APP_NAME="MyApplication"
//! export DATABASE_PORT="5432"
//! export ENABLE_DEBUG="true"
//! export API_TIMEOUT="30.5"
//!
//! # Run the example
//! cargo run --example basic_usage --features env
//! ```

use configuration::prelude::*;

fn main() -> Result<()> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    println!("=== Configuration Crate: Basic Usage ===\n");

    // Create a configuration service with environment variables
    // The builder pattern makes it easy to configure the service
    let service = DefaultConfigService::builder()
        .with_env_vars()
        .build()?;

    println!("Configuration service created with environment variables.\n");

    // Example 1: Get a string value
    println!("--- Example 1: String Values ---");
    let app_name_key = ConfigKey::from("app.name");

    match service.get(&app_name_key) {
        Ok(value) => {
            println!("✓ APP_NAME found: {}", value.as_str());
        }
        Err(_) => {
            println!("✗ APP_NAME not found, using default");
            let default_value = service.get_or_default(&app_name_key, "DefaultApp");
            println!("  Default value: {}", default_value.as_str());
        }
    }

    // Example 2: Get an integer value with type conversion
    println!("\n--- Example 2: Integer Values ---");
    let port_key = ConfigKey::from("database.port");

    match service.get(&port_key) {
        Ok(value) => {
            match value.as_i32("database.port") {
                Ok(port) => println!("✓ DATABASE_PORT found: {} (as i32)", port),
                Err(e) => println!("✗ DATABASE_PORT found but conversion failed: {}", e),
            }
        }
        Err(_) => {
            println!("✗ DATABASE_PORT not found, using default: 3000");
        }
    }

    // Example 3: Get a boolean value
    println!("\n--- Example 3: Boolean Values ---");
    let debug_key = ConfigKey::from("enable.debug");

    match service.get(&debug_key) {
        Ok(value) => {
            match value.as_bool("enable.debug") {
                Ok(enabled) => println!("✓ ENABLE_DEBUG found: {} (as bool)", enabled),
                Err(e) => println!("✗ ENABLE_DEBUG found but conversion failed: {}", e),
            }
        }
        Err(_) => {
            println!("✗ ENABLE_DEBUG not found, using default: false");
        }
    }

    // Example 4: Get a float value
    println!("\n--- Example 4: Float Values ---");
    let timeout_key = ConfigKey::from("api.timeout");

    match service.get(&timeout_key) {
        Ok(value) => {
            match value.as_f64("api.timeout") {
                Ok(timeout) => println!("✓ API_TIMEOUT found: {} seconds (as f64)", timeout),
                Err(e) => println!("✗ API_TIMEOUT found but conversion failed: {}", e),
            }
        }
        Err(_) => {
            println!("✗ API_TIMEOUT not found, using default: 10.0 seconds");
        }
    }

    // Example 5: Check if a key exists
    println!("\n--- Example 5: Checking Key Existence ---");
    let test_key = ConfigKey::from("some.random.key");

    if service.has(&test_key) {
        println!("✓ Key 'some.random.key' exists");
    } else {
        println!("✗ Key 'some.random.key' does not exist");
    }

    // Example 6: Using get_or_default for optional configuration
    println!("\n--- Example 6: Optional Configuration with Defaults ---");
    let log_level_key = ConfigKey::from("log.level");
    let log_level = service.get_or_default(&log_level_key, "info");
    println!("Log level: {} (from LOG_LEVEL or default)", log_level.as_str());

    println!("\n=== Example Complete ===");
    println!("\nTip: Try setting different environment variables and running again!");
    println!("Examples:");
    println!("  export APP_NAME='My Cool App'");
    println!("  export DATABASE_PORT=8080");
    println!("  export ENABLE_DEBUG=false");

    Ok(())
}
