# Docker-Based Integration Testing

This document explains the Docker-based integration testing setup for the Redis and etcd adapters.

## Overview

The configuration crate includes integration tests that:
- ✅ Use real Redis and etcd containers via Docker
- ✅ Automatically detect Docker availability
- ✅ Skip gracefully with warnings if Docker is not available
- ✅ Clean up containers automatically after tests
- ✅ Work with standard `cargo test` - no special tools required

## Implementation

### Architecture

```
tests/
├── docker_helpers.rs          # Docker detection utilities
├── redis_integration_tests.rs # Redis adapter tests
└── etcd_integration_tests.rs  # etcd adapter tests
```

### Key Components

#### 1. Docker Detection (`docker_helpers.rs`)

```rust
pub fn is_docker_available() -> bool {
    // Checks once, caches result
    // Uses `docker ps` command
}

pub fn print_docker_unavailable_warning(test_name: &str) {
    // Prints helpful warning message
}
```

#### 2. Test Structure

Each test follows this pattern:

```rust
#[tokio::test]
async fn test_something() {
    // 1. Check Docker availability
    if !docker_helpers::is_docker_available() {
        docker_helpers::print_docker_unavailable_warning("test name");
        return;  // Skip test, don't fail
    }

    // 2. Start container with testcontainers
    let docker = clients::Cli::default();
    let container = docker.run(Redis::default());

    // 3. Run test
    // ...

    // 4. Container auto-cleanup when dropped
}
```

## Running Tests

### Without Docker

```bash
$ cargo test
...
⚠️  SKIPPED: Redis integration test - Docker is not available
   To run this test, ensure Docker is installed and running.
   Installation: https://docs.docker.com/get-docker/

test result: ok. 125 passed; 0 failed; 0 ignored; 0 measured
```

Tests pass, but Docker tests are skipped with clear warnings.

### With Docker

```bash
# Terminal 1: Start Docker daemon
$ sudo systemctl start docker

# Terminal 2: Run tests
$ cargo test --features redis
...
test redis_tests::test_redis_hash_mode_get ... ok
test redis_tests::test_redis_string_keys_mode_get ... ok
test redis_tests::test_redis_reload ... ok
...
test result: ok. 133 passed; 0 failed; 0 ignored
```

All tests run including Docker-based ones.

## Benefits of This Approach

### 1. Zero Special Tooling

- ✅ Standard `cargo test` - no custom commands
- ✅ Works in any Rust environment
- ✅ No build scripts or custom test runners
- ✅ IDE test integration works out of the box

### 2. Flexible Execution

- ✅ Developers without Docker can run most tests
- ✅ CI can run basic tests without Docker setup
- ✅ Full tests when Docker is available
- ✅ No test failures due to missing Docker

### 3. Clear Feedback

```
⚠️  SKIPPED: etcd integration test - Docker is not available
   To run this test, ensure Docker is installed and running.
   Installation: https://docs.docker.com/get-docker/
```

Users immediately know:
- Why the test was skipped
- What they need to do to enable it
- Where to get Docker

## Alternative: cargo-xtask Pattern

While not needed for this use case, `cargo xtask` is an alternative for complex test scenarios:

### When to Use cargo-xtask

- Complex multi-step test workflows
- Need to compile test infrastructure
- Custom test runners with special logic
- Orchestrating multiple services

### Example Structure

```
project/
├── Cargo.toml
├── xtask/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs  # Custom test commands
└── src/
    └── lib.rs
```

### Example xtask

```rust
// xtask/src/main.rs
use std::process::Command;

fn main() {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("test-docker") => {
            // Start Docker containers
            // Run tests
            // Clean up
        }
        Some("test-all") => {
            // Run all test suites
        }
        _ => {
            println!("Available tasks:");
            println!("  cargo xtask test-docker");
            println!("  cargo xtask test-all");
        }
    }
}
```

### Usage

```bash
cargo xtask test-docker
```

### Why We Didn't Use xtask

For this project, `testcontainers` with feature detection is simpler:

| Aspect | testcontainers | cargo-xtask |
|--------|----------------|-------------|
| Setup | Add dev-dependency | Create xtask workspace |
| Code | Tests look normal | Custom runner code |
| IDE | Works automatically | Requires configuration |
| CI | Standard cargo test | Custom commands |
| Learning curve | Low | Medium |
| Flexibility | High enough | Very high |

## Comparison: testcontainers vs Manual Docker

### testcontainers (Our Approach)

```rust
let docker = clients::Cli::default();
let container = docker.run(Redis::default());
let port = container.get_host_port_ipv4(6379);
// Container auto-cleanup
```

**Pros:**
- Automatic container lifecycle
- Type-safe container configuration
- Guaranteed cleanup (RAII)
- Port conflict handling
- Works on all platforms

**Cons:**
- Requires testcontainers crate
- Some overhead

### Manual Docker (Alternative)

```rust
std::process::Command::new("docker")
    .args(["run", "-d", "-p", "6379:6379", "redis"])
    .output()?;

// ... test ...

std::process::Command::new("docker")
    .args(["stop", container_id])
    .output()?;
```

**Pros:**
- No extra dependencies
- Full control

**Cons:**
- Manual cleanup required
- Error-prone
- Port conflict management needed
- Cleanup on panic/failure is hard
- Platform-specific issues

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Tests

on: [push, pull_request]

jobs:
  # Fast tests without Docker
  test-basic:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cargo test

  # Full tests with Docker
  test-docker:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1

      # Docker is pre-installed on GitHub Actions runners

      - name: Install protoc (for etcd)
        run: sudo apt-get install -y protobuf-compiler

      - name: Run Redis tests
        run: cargo test --features redis

      - name: Run etcd tests
        run: cargo test --features etcd
```

### GitLab CI Example

```yaml
test:basic:
  script:
    - cargo test

test:docker:
  image: rust:latest
  services:
    - docker:dind
  variables:
    DOCKER_HOST: tcp://docker:2375
  before_script:
    - apt-get update && apt-get install -y protobuf-compiler
  script:
    - cargo test --features redis
    - cargo test --features etcd
```

## Testing the Tests

To verify Docker detection works:

```bash
# Test with Docker available
cargo test --features redis -- --nocapture

# Test without Docker (stop Docker first)
sudo systemctl stop docker
cargo test --features redis -- --nocapture
# Should see skip warnings

# Restart Docker
sudo systemctl start docker
```

## Troubleshooting

### Tests hang

**Cause**: Docker daemon not responding.

**Solution**:
```bash
# Restart Docker
sudo systemctl restart docker

# Check status
docker ps
```

### Container port conflicts

**Cause**: Port already in use.

**testcontainers handles this automatically** by using random host ports. You don't need to worry about conflicts.

### Permission denied

**Cause**: User not in docker group.

**Solution**:
```bash
sudo usermod -aG docker $USER
newgrp docker
```

## Best Practices

### 1. Keep Tests Isolated

Each test starts fresh containers - no shared state.

### 2. Use Reasonable Timeouts

```rust
// Give containers time to start
tokio::time::sleep(Duration::from_millis(500)).await;
```

### 3. Test Real Scenarios

Use containers to test:
- Actual network communication
- Real serialization
- Error handling (network failures, etc.)

### 4. Don't Test Container Startup

Test your code, not testcontainers:

```rust
// ❌ Don't test if container starts
assert!(container_started);

// ✅ Test your adapter works
assert_eq!(adapter.get(&key)?, expected_value);
```

## Summary

This setup provides:
- ✅ Real integration testing with Docker
- ✅ Graceful fallback without Docker
- ✅ Standard tooling (cargo test)
- ✅ Clear user feedback
- ✅ Easy CI/CD integration
- ✅ Low maintenance overhead

The key insight: **Use feature detection instead of mandatory Docker**. This makes tests more accessible while still providing thorough testing when possible.
