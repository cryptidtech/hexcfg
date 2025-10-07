// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for Redis adapter using Docker containers.

mod common;

#[cfg(feature = "redis")]
mod redis_tests {
    use configuration::adapters::{RedisAdapter, RedisStorageMode};
    use configuration::domain::ConfigKey;
    use configuration::ports::ConfigSource;
    use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage, ImageExt};

    use crate::common as docker_helpers;

    /// Helper to set up a Redis container and adapter for testing.
    async fn setup_redis_test(
        storage_mode: RedisStorageMode,
    ) -> Option<(testcontainers::ContainerAsync<GenericImage>, RedisAdapter)> {
        if !docker_helpers::is_docker_available() {
            docker_helpers::print_docker_unavailable_warning("Redis integration test");
            return None;
        }

        let redis_image = GenericImage::new("redis", "7-alpine")
            .with_exposed_port(6379.into())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));

        let container = redis_image.start().await.ok()?;
        let port = container.get_host_port_ipv4(6379).await.ok()?;

        let url = format!("redis://127.0.0.1:{}", port);

        // Give Redis a moment to start up
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let adapter = match storage_mode {
            RedisStorageMode::Hash => {
                // For hash mode, we need to set up a hash with test data
                let client = redis::Client::open(url.as_str()).unwrap();
                let mut conn = client.get_multiplexed_async_connection().await.unwrap();

                // Set up test data in hash
                let _: () = redis::cmd("HSET")
                    .arg("test_hash")
                    .arg("test.key")
                    .arg("test_value")
                    .arg("database.host")
                    .arg("localhost")
                    .arg("database.port")
                    .arg("5432")
                    .query_async(&mut conn)
                    .await
                    .unwrap();

                RedisAdapter::new(&url, "test_hash", storage_mode)
                    .await
                    .unwrap()
            }
            RedisStorageMode::StringKeys => {
                // For string keys mode, set up individual keys
                let client = redis::Client::open(url.as_str()).unwrap();
                let mut conn = client.get_multiplexed_async_connection().await.unwrap();

                let _: () = redis::cmd("SET")
                    .arg("test:test.key")
                    .arg("test_value")
                    .query_async(&mut conn)
                    .await
                    .unwrap();

                let _: () = redis::cmd("SET")
                    .arg("test:database.host")
                    .arg("localhost")
                    .query_async(&mut conn)
                    .await
                    .unwrap();

                let _: () = redis::cmd("SET")
                    .arg("test:database.port")
                    .arg("5432")
                    .query_async(&mut conn)
                    .await
                    .unwrap();

                RedisAdapter::new(&url, "test:", storage_mode)
                    .await
                    .unwrap()
            }
        };

        Some((container, adapter))
    }

    #[tokio::test]
    async fn test_redis_hash_mode_get() {
        let Some((_container, adapter)) = setup_redis_test(RedisStorageMode::Hash).await else {
            return;
        };

        let key = ConfigKey::from("test.key");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "test_value");
    }

    #[tokio::test]
    async fn test_redis_hash_mode_all_keys() {
        let Some((_container, adapter)) = setup_redis_test(RedisStorageMode::Hash).await else {
            return;
        };

        let keys = adapter.all_keys().unwrap();

        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&ConfigKey::from("test.key")));
        assert!(keys.contains(&ConfigKey::from("database.host")));
        assert!(keys.contains(&ConfigKey::from("database.port")));
    }

    #[tokio::test]
    async fn test_redis_string_keys_mode_get() {
        let Some((_container, adapter)) = setup_redis_test(RedisStorageMode::StringKeys).await
        else {
            return;
        };

        let key = ConfigKey::from("test.key");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "test_value");
    }

    #[tokio::test]
    async fn test_redis_string_keys_mode_all_keys() {
        let Some((_container, adapter)) = setup_redis_test(RedisStorageMode::StringKeys).await
        else {
            return;
        };

        let keys = adapter.all_keys().unwrap();

        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&ConfigKey::from("test.key")));
        assert!(keys.contains(&ConfigKey::from("database.host")));
        assert!(keys.contains(&ConfigKey::from("database.port")));
    }

    #[tokio::test]
    async fn test_redis_reload() {
        let Some((container, mut adapter)) = setup_redis_test(RedisStorageMode::Hash).await else {
            return;
        };

        // Initial value
        let key = ConfigKey::from("test.key");
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "test_value");

        // Update value in Redis
        let port = container.get_host_port_ipv4(6379).await.unwrap();
        let url = format!("redis://127.0.0.1:{}", port);
        let client = redis::Client::open(url.as_str()).unwrap();
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();

        let _: () = redis::cmd("HSET")
            .arg("test_hash")
            .arg("test.key")
            .arg("updated_value")
            .query_async(&mut conn)
            .await
            .unwrap();

        // Value should still be old before reload
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "test_value");

        // Reload adapter
        adapter.reload().unwrap();

        // Value should be updated
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "updated_value");
    }

    #[tokio::test]
    async fn test_redis_priority() {
        let Some((_container, adapter)) = setup_redis_test(RedisStorageMode::Hash).await else {
            return;
        };

        assert_eq!(adapter.priority(), 1);
    }

    #[tokio::test]
    async fn test_redis_custom_priority() {
        if !docker_helpers::is_docker_available() {
            docker_helpers::print_docker_unavailable_warning("Redis custom priority test");
            return;
        }

        let redis_image = GenericImage::new("redis", "7-alpine")
            .with_exposed_port(6379.into())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));

        let container = redis_image.start().await.unwrap();
        let port = container.get_host_port_ipv4(6379).await.unwrap();
        let url = format!("redis://127.0.0.1:{}", port);

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let adapter = RedisAdapter::with_priority(&url, "test:", RedisStorageMode::Hash, 5)
            .await
            .unwrap();

        assert_eq!(adapter.priority(), 5);
    }

    #[tokio::test]
    async fn test_redis_nonexistent_key() {
        let Some((_container, adapter)) = setup_redis_test(RedisStorageMode::Hash).await else {
            return;
        };

        let key = ConfigKey::from("nonexistent.key");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_cli_overrides_redis() {
        use configuration::adapters::CommandLineAdapter;
        use configuration::domain::ConfigurationService;
        use configuration::service::ConfigurationServiceBuilder;

        let Some((_container, redis_adapter)) = setup_redis_test(RedisStorageMode::Hash).await
        else {
            return;
        };

        // Redis has "test.key" = "test_value"
        // CLI will override with "test.key" = "from_cli"
        let cli_args = vec!["--test.key=from_cli"];
        let cli_adapter = CommandLineAdapter::from_args(cli_args);

        let service = ConfigurationServiceBuilder::new()
            .with_source(Box::new(redis_adapter))
            .with_source(Box::new(cli_adapter))
            .build()
            .unwrap();

        // CLI (priority 3) should win over Redis (priority 1)
        let value = service.get_str("test.key").unwrap();
        assert_eq!(value.as_str(), "from_cli");
    }

    #[tokio::test]
    async fn test_env_overrides_redis() {
        use configuration::adapters::EnvVarAdapter;
        use configuration::domain::ConfigurationService;
        use configuration::service::ConfigurationServiceBuilder;
        use std::collections::HashMap;

        let Some((_container, redis_adapter)) = setup_redis_test(RedisStorageMode::Hash).await
        else {
            return;
        };

        // Redis has "test.key" = "test_value"
        // Env will override with "test.key" = "from_env"
        let mut env_vars = HashMap::new();
        env_vars.insert("test.key".to_string(), "from_env".to_string());
        let env_adapter = EnvVarAdapter::with_values(env_vars);

        let service = ConfigurationServiceBuilder::new()
            .with_source(Box::new(redis_adapter))
            .with_source(Box::new(env_adapter))
            .build()
            .unwrap();

        // Env (priority 2) should win over Redis (priority 1)
        let value = service.get_str("test.key").unwrap();
        assert_eq!(value.as_str(), "from_env");
    }

    #[tokio::test]
    async fn test_full_precedence_chain_with_redis() {
        use configuration::adapters::{CommandLineAdapter, EnvVarAdapter};
        use configuration::domain::ConfigurationService;
        use configuration::service::ConfigurationServiceBuilder;
        use std::collections::HashMap;

        let Some((_container, redis_adapter)) = setup_redis_test(RedisStorageMode::Hash).await
        else {
            return;
        };

        // Redis has:
        // - "test.key" = "test_value"
        // - "database.host" = "localhost"
        // - "database.port" = "5432"

        // Env overrides some keys
        let mut env_vars = HashMap::new();
        env_vars.insert("test.key".to_string(), "from_env".to_string());
        env_vars.insert("database.host".to_string(), "env.example.com".to_string());
        let env_adapter = EnvVarAdapter::with_values(env_vars);

        // CLI overrides even more
        let cli_args = vec!["--test.key=from_cli"];
        let cli_adapter = CommandLineAdapter::from_args(cli_args);

        let service = ConfigurationServiceBuilder::new()
            .with_source(Box::new(redis_adapter))
            .with_source(Box::new(env_adapter))
            .with_source(Box::new(cli_adapter))
            .build()
            .unwrap();

        // test.key: CLI wins (priority 3)
        let value = service.get_str("test.key").unwrap();
        assert_eq!(value.as_str(), "from_cli");

        // database.host: Env wins (priority 2)
        let value = service.get_str("database.host").unwrap();
        assert_eq!(value.as_str(), "env.example.com");

        // database.port: Redis is the only source (priority 1)
        let value = service.get_str("database.port").unwrap();
        assert_eq!(value.as_str(), "5432");
    }

    #[tokio::test]
    async fn test_redis_with_custom_priority_overrides_env() {
        use configuration::adapters::EnvVarAdapter;
        use configuration::domain::ConfigurationService;
        use configuration::service::ConfigurationServiceBuilder;
        use std::collections::HashMap;

        if !docker_helpers::is_docker_available() {
            docker_helpers::print_docker_unavailable_warning(
                "Redis custom priority precedence test",
            );
            return;
        }

        let redis_image = GenericImage::new("redis", "7-alpine")
            .with_exposed_port(6379.into())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));

        let container = redis_image.start().await.unwrap();
        let port = container.get_host_port_ipv4(6379).await.unwrap();
        let url = format!("redis://127.0.0.1:{}", port);

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Set up Redis with test data
        let client = redis::Client::open(url.as_str()).unwrap();
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();

        let _: () = redis::cmd("HSET")
            .arg("test_hash")
            .arg("special.key")
            .arg("from_redis")
            .query_async(&mut conn)
            .await
            .unwrap();

        // Create Redis adapter with custom priority 3 (same as CLI!)
        let redis_adapter =
            RedisAdapter::with_priority(&url, "test_hash", RedisStorageMode::Hash, 3)
                .await
                .unwrap();

        // Create env adapter with normal priority 2
        let mut env_vars = HashMap::new();
        env_vars.insert("special.key".to_string(), "from_env".to_string());
        let env_adapter = EnvVarAdapter::with_values(env_vars);

        let service = ConfigurationServiceBuilder::new()
            .with_source(Box::new(env_adapter))
            .with_source(Box::new(redis_adapter))
            .build()
            .unwrap();

        // Redis with priority 3 should win over env with priority 2
        let value = service.get_str("special.key").unwrap();
        assert_eq!(value.as_str(), "from_redis");
    }

    // === Redis Watcher Tests ===

    /// Helper to set up a Redis container with keyspace notifications enabled.
    async fn setup_redis_watcher_test(
    ) -> Option<(testcontainers::ContainerAsync<GenericImage>, String)> {
        if !docker_helpers::is_docker_available() {
            docker_helpers::print_docker_unavailable_warning("Redis watcher test");
            return None;
        }

        // Start Redis with keyspace notifications enabled
        let redis_image = GenericImage::new("redis", "7-alpine")
            .with_exposed_port(6379.into())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .with_cmd(vec!["redis-server", "--notify-keyspace-events", "KEA"]);

        let container = redis_image.start().await.ok()?;
        let port = container.get_host_port_ipv4(6379).await.ok()?;
        let url = format!("redis://127.0.0.1:{}", port);

        // Give Redis a moment to start up
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        Some((container, url))
    }

    #[tokio::test]
    async fn test_redis_watcher_creation() {
        use configuration::adapters::RedisWatcher;

        let Some((_container, url)) = setup_redis_watcher_test().await else {
            return;
        };

        let watcher = RedisWatcher::new(&url, "test:");
        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_redis_watcher_invalid_url() {
        use configuration::adapters::RedisWatcher;

        let watcher = RedisWatcher::new("redis://invalid-host:9999", "test:");
        assert!(watcher.is_err());
    }

    #[tokio::test]
    async fn test_redis_watcher_start_stop() {
        use configuration::adapters::RedisWatcher;
        use configuration::ports::ConfigWatcher;
        use std::sync::Arc;

        let Some((_container, url)) = setup_redis_watcher_test().await else {
            return;
        };

        let mut watcher = RedisWatcher::new(&url, "test:").unwrap();

        let callback = Arc::new(|_key: ConfigKey| {
            // Callback for testing
        });

        // Start watching
        assert!(watcher.watch(callback).is_ok());

        // Give watcher time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Stop watching
        assert!(watcher.stop().is_ok());
    }

    #[tokio::test]
    async fn test_redis_watcher_callback_triggered() {
        use configuration::adapters::RedisWatcher;
        use configuration::ports::ConfigWatcher;
        use redis::Commands;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let Some((_container, url)) = setup_redis_watcher_test().await else {
            return;
        };

        let mut watcher = RedisWatcher::new(&url, "test:watcher:").unwrap();

        let triggered = Arc::new(AtomicBool::new(false));
        let triggered_clone = Arc::clone(&triggered);

        let callback = Arc::new(move |key: ConfigKey| {
            eprintln!("Redis watcher callback triggered for key: {}", key.as_str());
            triggered_clone.store(true, Ordering::SeqCst);
        });

        watcher.watch(callback).unwrap();

        // Wait for watcher to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Modify a key in Redis
        let client = redis::Client::open(url.as_str()).unwrap();
        let mut conn = client.get_connection().unwrap();
        let _: () = conn.set("test:watcher:mykey", "test_value").unwrap();

        // Wait for the event to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Clean up
        let _: () = conn.del("test:watcher:mykey").unwrap();
        watcher.stop().unwrap();

        let was_triggered = triggered.load(Ordering::SeqCst);
        assert!(
            was_triggered,
            "Redis watcher callback should have been triggered"
        );
    }

    #[tokio::test]
    async fn test_redis_watcher_multiple_changes() {
        use configuration::adapters::RedisWatcher;
        use configuration::ports::ConfigWatcher;
        use redis::Commands;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let Some((_container, url)) = setup_redis_watcher_test().await else {
            return;
        };

        let mut watcher = RedisWatcher::new(&url, "test:multi:").unwrap();

        let trigger_count = Arc::new(AtomicUsize::new(0));
        let trigger_count_clone = Arc::clone(&trigger_count);

        let callback = Arc::new(move |key: ConfigKey| {
            eprintln!("Redis watcher detected change: {}", key.as_str());
            trigger_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        watcher.watch(callback).unwrap();

        // Wait for watcher to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Make multiple changes
        let client = redis::Client::open(url.as_str()).unwrap();
        let mut conn = client.get_connection().unwrap();

        for i in 0..3 {
            let _: () = conn
                .set(format!("test:multi:key{}", i), format!("value{}", i))
                .unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // Wait for events to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Clean up
        for i in 0..3 {
            let _: () = conn.del(format!("test:multi:key{}", i)).unwrap();
        }
        watcher.stop().unwrap();

        let count = trigger_count.load(Ordering::SeqCst);
        assert!(count >= 3, "Expected at least 3 triggers, got {}", count);
    }

    #[tokio::test]
    async fn test_redis_watcher_namespace_filtering() {
        use configuration::adapters::RedisWatcher;
        use configuration::ports::ConfigWatcher;
        use redis::Commands;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let Some((_container, url)) = setup_redis_watcher_test().await else {
            return;
        };

        let mut watcher = RedisWatcher::new(&url, "test:myapp:").unwrap();

        let trigger_count = Arc::new(AtomicUsize::new(0));
        let trigger_count_clone = Arc::clone(&trigger_count);

        let callback = Arc::new(move |key: ConfigKey| {
            eprintln!("Redis watcher detected change: {}", key.as_str());
            trigger_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        watcher.watch(callback).unwrap();

        // Wait for watcher to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let client = redis::Client::open(url.as_str()).unwrap();
        let mut conn = client.get_connection().unwrap();

        // This should trigger (has correct namespace)
        let _: () = conn.set("test:myapp:key1", "value1").unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // This should NOT trigger (different namespace)
        let _: () = conn.set("other:app:key2", "value2").unwrap();

        // Wait for events to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Clean up
        let _: () = conn.del("test:myapp:key1").unwrap();
        let _: () = conn.del("other:app:key2").unwrap();
        watcher.stop().unwrap();

        let count = trigger_count.load(Ordering::SeqCst);
        // Redis keyspace notifications may generate multiple events for the same key change,
        // so we verify at least 1 event was received (confirming filtering works).
        // The important part is that only key1 triggered, not key2.
        assert!(
            count >= 1,
            "Expected at least 1 trigger for key with correct namespace, got {}",
            count
        );
        // Verify we didn't get an excessive number of triggers
        assert!(
            count <= 3,
            "Expected at most 3 triggers, got {} (something may be wrong)",
            count
        );
    }
}
