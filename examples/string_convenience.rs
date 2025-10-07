// SPDX-License-Identifier: MIT OR Apache-2.0

//! String convenience methods example.
//!
//! This example demonstrates the convenience methods for using string slices
//! directly without having to manually create ConfigKey instances.
//!
//! To run this example:
//! ```bash
//! # Set some environment variables
//! export APP_NAME="MyApp"
//! export APP_PORT="8080"
//! export ENABLE_CACHE="true"
//!
//! # Run the example
//! cargo run --example string_convenience --features env
//! ```

use configuration::prelude::*;

fn main() -> Result<()> {
    println!("=== String Convenience Methods Example ===\n");

    // Create a configuration service
    let service = DefaultConfigService::builder().with_env_vars().build()?;

    println!("Demonstrating the difference between regular and convenience methods:\n");

    // OLD WAY: Using ConfigKey explicitly
    println!("--- Traditional Approach (with ConfigKey) ---");
    let key = ConfigKey::from("app.name");
    match service.get(&key) {
        Ok(value) => println!("✓ get(&ConfigKey::from(\"app.name\")): {}", value.as_str()),
        Err(_) => println!("✗ Key not found"),
    }

    // NEW WAY: Using string slice directly
    println!("\n--- Convenient Approach (with string slice) ---");
    match service.get_str("app.name") {
        Ok(value) => println!("✓ get_str(\"app.name\"): {}", value.as_str()),
        Err(_) => println!("✗ Key not found"),
    }

    // Demonstrating all convenience methods
    println!("\n=== All Convenience Methods ===\n");

    // get_str
    println!("1. get_str(\"app.port\")");
    match service.get_str("app.port") {
        Ok(value) => {
            println!("   Found: {}", value.as_str());
            if let Ok(port) = value.as_i32("app.port") {
                println!("   As integer: {}", port);
            }
        }
        Err(_) => println!("   Not found"),
    }

    // get_or_default_str
    println!("\n2. get_or_default_str(\"log.level\", \"info\")");
    let log_level = service.get_or_default_str("log.level", "info");
    println!(
        "   Value: {} (from LOG_LEVEL or default)",
        log_level.as_str()
    );

    // has_str
    println!("\n3. has_str(\"enable.cache\")");
    if service.has_str("enable.cache") {
        println!("   ✓ Key exists");
        let value = service.get_str("enable.cache")?;
        if let Ok(enabled) = value.as_bool("enable.cache") {
            println!("   Cache enabled: {}", enabled);
        }
    } else {
        println!("   ✗ Key does not exist");
    }

    println!("\n4. has_str(\"nonexistent.key\")");
    if service.has_str("nonexistent.key") {
        println!("   ✓ Key exists");
    } else {
        println!("   ✗ Key does not exist");
    }

    // Direct comparison
    println!("\n=== Code Comparison ===\n");
    println!("Before (verbose):");
    println!("  let key = ConfigKey::from(\"app.name\");");
    println!("  let value = service.get(&key)?;");
    println!();
    println!("After (concise):");
    println!("  let value = service.get_str(\"app.name\")?;");
    println!();
    println!("Both approaches work! Use whichever feels more natural.");

    // ConfigSource also has get_str
    println!("\n=== ConfigSource Convenience Methods ===\n");
    println!("Note: ConfigSource trait also has get_str() for individual sources.");
    println!("Example usage with an adapter:");
    println!("  let adapter = EnvVarAdapter::new();");
    println!("  let value = adapter.get_str(\"app.name\")?;");

    println!("\n=== Example Complete ===");

    Ok(())
}
