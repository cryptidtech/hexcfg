// SPDX-License-Identifier: MIT OR Apache-2.0

//! Helper utilities for Docker-based integration tests.

use std::sync::OnceLock;

/// Cached result of Docker availability check.
#[allow(dead_code)]
static DOCKER_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Checks if Docker is available on the system.
///
/// This check is cached after the first call.
#[allow(dead_code)]
pub fn is_docker_available() -> bool {
    *DOCKER_AVAILABLE.get_or_init(|| {
        // Try to run `docker ps` command
        std::process::Command::new("docker")
            .args(["ps"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    })
}

/// Prints a warning message that a test is skipped due to Docker being unavailable.
#[allow(dead_code)]
pub fn print_docker_unavailable_warning(test_name: &str) {
    eprintln!("\n⚠️  SKIPPED: {} - Docker is not available", test_name);
    eprintln!("   To run this test, ensure Docker is installed and running.");
    eprintln!("   Installation: https://docs.docker.com/get-docker/\n");
}

/// Macro to skip test if Docker is unavailable with a warning.
#[macro_export]
macro_rules! require_docker {
    ($test_name:expr) => {
        if !$crate::docker_helpers::is_docker_available() {
            $crate::docker_helpers::print_docker_unavailable_warning($test_name);
            return;
        }
    };
}
