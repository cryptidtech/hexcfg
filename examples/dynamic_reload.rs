// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dynamic configuration reload example.
//!
//! This example demonstrates:
//! - Watching configuration files for changes
//! - Automatically reloading configuration when files change
//! - Using callbacks to react to configuration changes
//! - Debouncing to avoid excessive reloads
//!
//! To run this example:
//! ```bash
//! cargo run --example dynamic_reload --features yaml,reload
//!
//! # In another terminal, modify the config file:
//! echo "app:
//!   name: UpdatedApp
//!   port: 9000" > /tmp/watched_config.yaml
//! ```

#[cfg(all(feature = "reload", feature = "yaml"))]
use hexcfg::prelude::*;
#[cfg(all(feature = "reload", feature = "yaml"))]
use std::sync::{Arc, Mutex};
#[cfg(all(feature = "reload", feature = "yaml"))]
use std::thread;
#[cfg(all(feature = "reload", feature = "yaml"))]
use std::time::Duration;

#[cfg(all(feature = "reload", feature = "yaml"))]
fn main() -> Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("=== Configuration Crate: Dynamic Reload Example ===\n");

    // Create a temporary YAML config file
    let yaml_content = r#"
app:
  name: "InitialApp"
  port: 8080
  environment: "development"

database:
  host: "localhost"
  port: 5432
  max_connections: 10

features:
  logging: true
  metrics: false
"#;

    let temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(temp_file.path(), yaml_content)?;

    println!("Created config file at: {:?}", temp_file.path());
    println!("Initial configuration:\n{}\n", yaml_content);

    // Create configuration service with a YAML file source
    let service = Arc::new(Mutex::new(
        ConfigurationServiceBuilder::new()
            .with_yaml_file(temp_file.path())?
            .build()?,
    ));

    // Display initial configuration values
    println!("=== Initial Configuration Values ===");
    print_config_values(&service.lock().unwrap());

    // Create a file watcher with a 1-second debounce delay
    let mut watcher = FileWatcher::new(temp_file.path(), Some(Duration::from_secs(1)))?;

    // Set up the change callback
    let service_clone = Arc::clone(&service);
    let callback = Arc::new(move |key: ConfigKey| {
        println!("\n🔄 Configuration change detected: {}", key);
        println!("Reloading configuration...");

        // Reload the configuration service
        if let Ok(mut svc) = service_clone.lock() {
            if let Err(e) = svc.reload() {
                eprintln!("Error reloading configuration: {}", e);
                return;
            }

            println!("\n=== Updated Configuration Values ===");
            print_config_values(&svc);
        }
    });

    // Start watching for changes
    println!("\n=== Starting Configuration Watcher ===");
    println!("Watching for changes to: {:?}", temp_file.path());
    println!("Debounce delay: 1 second");
    println!("\nTry modifying the configuration file in another terminal:");
    println!("  echo 'app:");
    println!("    name: UpdatedApp");
    println!("    port: 9000' > {:?}", temp_file.path());

    watcher.watch(callback)?;

    // Keep the application running for demonstration
    println!("\nApplication is running. Press Ctrl+C to exit.");
    println!("The application will automatically reload configuration when the file changes.\n");

    // Simulate application runtime
    // In a real application, this would be your main application logic
    for i in 1..=30 {
        thread::sleep(Duration::from_secs(2));
        print!(".");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        if i % 10 == 0 {
            println!();
        }
    }

    println!("\n\n=== Stopping Watcher ===");
    watcher.stop()?;

    println!("Example complete. Configuration watcher stopped.");

    Ok(())
}

/// Helper function to print current configuration values
#[cfg(all(feature = "reload", feature = "yaml"))]
fn print_config_values(service: &DefaultConfigService) {
    let keys = vec![
        ("app.name", "string"),
        ("app.port", "integer"),
        ("app.environment", "string"),
        ("database.host", "string"),
        ("database.port", "integer"),
        ("database.max_connections", "integer"),
        ("features.logging", "boolean"),
        ("features.metrics", "boolean"),
    ];

    for (key, value_type) in keys {
        let config_key = ConfigKey::from(key);
        match service.get(&config_key) {
            Ok(value) => {
                let formatted_value = match value_type {
                    "integer" => value
                        .as_i32(key)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|_| value.as_str().to_string()),
                    "boolean" => value
                        .as_bool(key)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|_| value.as_str().to_string()),
                    _ => value.as_str().to_string(),
                };
                println!("  {:<30} = {}", key, formatted_value);
            }
            Err(_) => {
                println!("  {:<30} = <not set>", key);
            }
        }
    }
}

#[cfg(not(all(feature = "reload", feature = "yaml")))]
fn main() {
    eprintln!("Error: This example requires the 'reload' and 'yaml' features.");
    eprintln!("Run with: cargo run --example dynamic_reload --features yaml,reload");
    std::process::exit(1);
}
