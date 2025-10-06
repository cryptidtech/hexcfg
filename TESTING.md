# Testing Guide

This document explains how to run tests for the configuration crate, including Docker-based integration tests.

## Running Tests

### Basic Tests (No Docker Required)

Run the default test suite without Docker dependencies:

```bash
cargo test
```

This runs:
- Unit tests for all adapters
- Integration tests for env vars, CLI, and YAML file sources
- Documentation tests
- Reload tests (with `reload` feature)

### Docker-Based Integration Tests

The crate includes integration tests for Redis and etcd adapters that use Docker containers. These tests:

1. **Automatically detect Docker availability**
2. **Skip gracefully if Docker is not installed** with a clear warning message
3. **Start containers, run tests, and clean up automatically**

#### Running Redis Integration Tests

```bash
# Requires Docker to be running
cargo test --features redis
```

#### Running etcd Integration Tests

**Note**: etcd tests require `protoc` (Protocol Buffers compiler) to be installed:

```bash
# On Debian/Ubuntu
sudo apt-get install protobuf-compiler

# On macOS
brew install protobuf

# On Fedora
sudo dnf install protobuf-compiler
```

Then run:

```bash
# Requires Docker and protoc to be installed
cargo test --features etcd
```

#### Running All Remote Integration Tests

```bash
# Requires Docker and protoc to be running
cargo test --features remote
```

### What Happens Without Docker?

If Docker is not installed or not running, the integration tests will:

1. **Detect the absence of Docker** at test runtime
2. **Skip the test** instead of failing
3. **Print a clear warning** to stderr:

```
⚠️  SKIPPED: Redis integration test - Docker is not available
   To run this test, ensure Docker is installed and running.
   Installation: https://docs.docker.com/get-docker/
```

This means:
- ✅ `cargo test` always succeeds, even without Docker
- ✅ CI/CD pipelines can run basic tests without Docker
- ✅ Developers without Docker can still run most tests
- ✅ Full integration tests only run when Docker is available

## Test Organization

```
tests/
├── docker_helpers.rs          # Docker detection and utilities
├── etcd_integration_tests.rs  # etcd adapter tests (requires Docker + protoc)
├── redis_integration_tests.rs # Redis adapter tests (requires Docker)
├── precedence_tests.rs         # Multi-source precedence tests
└── reload_tests.rs             # Dynamic reload tests
```

## CI/CD Recommendations

### Without Docker

```yaml
- name: Run basic tests
  run: cargo test
```

### With Docker

```yaml
- name: Install protoc (for etcd)
  run: sudo apt-get install -y protobuf-compiler

- name: Run all tests including Docker-based
  run: |
    cargo test
    cargo test --features redis
    cargo test --features etcd
```

## Development Workflow

### Local Development (without Docker)

```bash
# Run fast unit and integration tests
cargo test

# Watch mode for rapid development
cargo watch -x test
```

### Pre-Commit (with Docker)

```bash
# Run full test suite including Docker-based tests
./scripts/test-all.sh  # or manually:
cargo test --all-features
```

### CI Pipeline

```bash
# Stage 1: Fast tests (no Docker)
cargo test

# Stage 2: Docker-based tests (optional, in separate job)
cargo test --features redis
cargo test --features etcd
```

## Test Performance

- **Unit tests**: ~0.01s (125 tests)
- **Integration tests (no Docker)**: ~0.01s (13 tests)
- **Reload tests**: ~1.2s (11 tests, includes file watching delays)
- **Redis integration tests**: ~2-5s (8 tests, includes container startup)
- **etcd integration tests**: ~5-10s (9 tests, includes container startup)

## Troubleshooting

### "Docker is not available" warnings

**Cause**: Docker is not installed or the Docker daemon is not running.

**Solution**:
1. Install Docker: https://docs.docker.com/get-docker/
2. Start Docker daemon: `sudo systemctl start docker` (Linux) or start Docker Desktop (macOS/Windows)
3. Verify: `docker ps` should work without errors

### etcd build fails: "Could not find protoc"

**Cause**: Protocol Buffers compiler is not installed.

**Solution**:
```bash
# Debian/Ubuntu
sudo apt-get install protobuf-compiler

# macOS
brew install protobuf

# Fedora
sudo dnf install protobuf-compiler

# Or download from: https://github.com/protocolbuffers/protobuf/releases
```

### Tests hang or timeout

**Cause**: Container failed to start or network issues.

**Solution**:
1. Check Docker is running: `docker ps`
2. Pull images manually:
   ```bash
   docker pull redis:latest
   docker pull quay.io/coreos/etcd:v3.5.0
   ```
3. Check network connectivity to Docker Hub / Quay.io
4. Increase timeout in tests if on slow system

### Permission denied connecting to Docker

**Cause**: User not in `docker` group (Linux).

**Solution**:
```bash
sudo usermod -aG docker $USER
# Log out and back in, or:
newgrp docker
```

## Writing New Docker-Based Tests

To add new Docker-based tests:

```rust
#[cfg(feature = "your_feature")]
mod your_tests {
    #[path = "docker_helpers.rs"]
    mod docker_helpers;

    #[tokio::test]
    async fn test_something() {
        // Check Docker availability
        if !docker_helpers::is_docker_available() {
            docker_helpers::print_docker_unavailable_warning("your test name");
            return;
        }

        // Your test code with testcontainers...
    }
}
```

## Feature Flags

- `yaml` - YAML file adapter (default)
- `env` - Environment variable adapter (default)
- `cli` - Command-line argument adapter (default)
- `reload` - Dynamic reloading with file watching
- `etcd` - etcd adapter (requires protoc to build)
- `redis` - Redis adapter
- `remote` - All remote adapters (etcd + redis)
- `full` - All features enabled
