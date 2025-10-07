# Remote Configuration Watchers

This document describes the etcd and Redis watchers added to the configuration library.

## Overview

The configuration library now supports real-time change notifications for remote configuration sources (etcd and Redis), similar to the existing file watcher functionality. These watchers automatically detect when configuration values change in the remote stores and trigger callbacks to reload the configuration.

## Features

### etcd Watcher

- **Native Watch API**: Uses etcd's built-in watch mechanism for server-side change detection
- **Prefix Filtering**: Watch only keys with a specific prefix (e.g., `myapp/`)
- **Key Normalization**: Automatically converts etcd's slash-based keys to dot notation (e.g., `myapp/db/host` → `db.host`)
- **Automatic Reconnection**: Handles connection failures with automatic retry logic
- **Thread-based**: Runs in a dedicated thread for non-blocking operation

### Redis Watcher

- **Keyspace Notifications**: Uses Redis pub/sub keyspace notifications for change detection
- **Namespace Filtering**: Watch only keys with a specific namespace prefix (e.g., `myapp:`)
- **Configuration Helper**: Includes `try_enable_keyspace_notifications()` method to enable notifications
- **Automatic Reconnection**: Handles connection failures with automatic retry logic
- **Thread-based**: Runs in a dedicated thread for non-blocking operation

## Usage

### etcd Watcher

```rust
use hexcfg::prelude::*;
use hexcfg::adapters::EtcdWatcher;
use hexcfg::ports::ConfigWatcher;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<()> {
    // Create configuration service with etcd
    let service = Arc::new(Mutex::new(
        ConfigurationServiceBuilder::new()
            .with_etcd(vec!["localhost:2379"], Some("myapp/")).await?
            .build()?
    ));

    // Create etcd watcher
    let mut watcher = EtcdWatcher::new(
        vec!["localhost:2379"],
        Some("myapp/")
    ).await?;

    // Register callback
    let service_clone = Arc::clone(&service);
    watcher.watch(Arc::new(move |key| {
        println!("Configuration changed: {}", key);
        if let Ok(mut svc) = service_clone.lock() {
            let _ = svc.reload();
        }
    }))?;

    // Your application logic here...

    // Stop watching when done
    watcher.stop()?;

    Ok(())
}
```

### Redis Watcher

```rust
use hexcfg::prelude::*;
use hexcfg::adapters::RedisWatcher;
use hexcfg::ports::ConfigWatcher;
use std::sync::{Arc, Mutex};

fn main() -> Result<()> {
    // Create configuration service with Redis
    let service = Arc::new(Mutex::new(
        ConfigurationServiceBuilder::new()
            .with_redis(
                "redis://localhost:6379",
                "myapp:",
                RedisStorageMode::StringKeys
            ).await?
            .build()?
    ));

    // Create Redis watcher
    let mut watcher = RedisWatcher::new(
        "redis://localhost:6379",
        "myapp:"
    )?;

    // Try to enable keyspace notifications (requires CONFIG permission)
    let _ = watcher.try_enable_keyspace_notifications();

    // Register callback
    let service_clone = Arc::clone(&service);
    watcher.watch(Arc::new(move |key| {
        println!("Configuration changed: {}", key);
        if let Ok(mut svc) = service_clone.lock() {
            let _ = svc.reload();
        }
    }))?;

    // Your application logic here...

    // Stop watching when done
    watcher.stop()?;

    Ok(())
}
```

## Configuration Requirements

### etcd

No special configuration required. etcd's watch API is available by default.

### Redis

Redis keyspace notifications must be enabled on the Redis server:

```bash
# Via redis-cli
redis-cli CONFIG SET notify-keyspace-events KEA

# Or in redis.conf
notify-keyspace-events KEA
```

**Configuration options:**
- `K`: Keyspace events (published to `__keyspace@<db>__:<key>`)
- `E`: Keyevent events (published to `__keyevent@<db>__:<event>`)
- `A`: Alias for "g$lshzxe" (all events)

For the Redis watcher, you need at least `KE` or `KEA`.

## Testing

Integration tests for the remote watchers are included in their respective integration test files (`tests/redis_integration_tests.rs` and `tests/etcd_integration_tests.rs`). These tests use `testcontainers-rs` to automatically start and manage Docker containers for testing.

### Running Watcher Tests

```bash
# Run all Redis tests (including watcher tests)
cargo test --test redis_integration_tests --all-features

# Run all etcd tests (including watcher tests)
cargo test --test etcd_integration_tests --all-features

# Run only watcher tests
cargo test --test redis_integration_tests test_redis_watcher --all-features
cargo test --test etcd_integration_tests test_etcd_watcher --all-features
```

### Requirements

- **Docker**: Must be installed and running
- If Docker is not available, tests will automatically skip with a warning message
- No manual container management required - `testcontainers-rs` handles everything

### What Gets Tested

Each watcher has the following tests:

1. **Creation**: Verify watcher can be instantiated with valid configuration
2. **Invalid Connection**: Test error handling for invalid endpoints/URLs
3. **Start/Stop**: Verify lifecycle management works correctly
4. **Callback Triggering**: Test that callbacks fire when keys change
5. **Multiple Changes**: Verify multiple sequential changes are detected
6. **Prefix/Namespace Filtering**: Ensure only relevant keys trigger callbacks

## Implementation Details

### Thread Management

Both watchers spawn dedicated threads to monitor for changes without blocking the main application. The threads use:

- **Stop signals**: Channel-based communication for graceful shutdown
- **Periodic checking**: Regular checks for stop signal during event processing
- **Join handles**: Proper thread cleanup on `stop()` or `Drop`

### Error Handling

- **Connection failures**: Automatic reconnection with exponential backoff
- **Parse errors**: Logged but don't crash the watcher
- **Callback errors**: Caught and logged to prevent watcher crashes

### Key Transformations

- **etcd**: Converts slash-separated keys (`myapp/db/host`) to dot notation (`db.host`)
- **Redis**: Already uses colon-separated namespaces, strips the namespace prefix

## Performance Considerations

### etcd Watcher

- **Efficient**: Uses server-side filtering (prefix matching)
- **Low latency**: Native watch API provides immediate notifications
- **Scalable**: etcd designed for distributed configuration

### Redis Watcher

- **Pub/sub overhead**: Redis keyspace notifications use pub/sub, which has some overhead
- **Pattern matching**: Uses pattern subscription (`__keyspace@0__:namespace*`)
- **Network traffic**: Each key change generates a notification

## Limitations

### etcd Watcher

- Requires network access to etcd cluster
- Async initialization (requires `tokio` runtime)
- Thread spawning overhead

### Redis Watcher

- Requires keyspace notifications to be enabled (may need CONFIG permission)
- Keyspace notifications have Redis performance overhead
- Pattern subscriptions match all keys with the namespace prefix

## Security Considerations

- **etcd**: Supports TLS and authentication (pass via connection options)
- **Redis**: Supports password authentication (include in connection URL)
- **Callbacks**: User callbacks run in the watcher thread, keep them lightweight
- **Thread safety**: Callbacks must be `Send + Sync`

## Future Enhancements

Potential improvements for future versions:

1. **Configurable retry logic**: Allow customization of reconnection backoff
2. **Batch notifications**: Group multiple rapid changes into single callback
3. **Filtered callbacks**: Allow callback registration for specific key patterns
4. **Metrics**: Expose watcher statistics (connection state, event counts, etc.)
5. **TLS support**: Built-in configuration for secure connections

## Troubleshooting

### etcd Watcher Not Receiving Events

1. Check etcd connectivity: `etcdctl endpoint health`
2. Verify prefix matches your keys: `etcdctl get --prefix myapp/`
3. Check firewall rules for port 2379

### Redis Watcher Not Receiving Events

1. Verify keyspace notifications are enabled:
   ```bash
   redis-cli CONFIG GET notify-keyspace-events
   ```
   Should return `KEA` or similar.

2. Test manual notification:
   ```bash
   redis-cli SET myapp:test value
   redis-cli SUBSCRIBE '__keyspace@0__:myapp:*'
   # In another terminal: redis-cli SET myapp:test value2
   ```

3. Check Redis logs for pub/sub issues

### High CPU Usage

If watchers consume too much CPU:

1. Ensure callbacks are lightweight (don't reload config on every change)
2. Consider debouncing or rate limiting in your callback
3. Check for connection issues causing rapid reconnection attempts

## API Reference

See the full API documentation:

```bash
cargo doc --open --all-features
```

Navigate to:
- `configuration::adapters::EtcdWatcher`
- `configuration::adapters::RedisWatcher`
- `configuration::ports::ConfigWatcher`
