// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for Redis adapter using Docker containers.

mod common;

#[cfg(feature = "redis")]
mod redis_tests {
    use configuration::adapters::{RedisAdapter, RedisStorageMode};
    use configuration::domain::ConfigKey;
    use configuration::ports::ConfigSource;
    use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage};

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

        let Some((_container, redis_adapter)) = setup_redis_test(RedisStorageMode::Hash).await else {
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

        let Some((_container, redis_adapter)) = setup_redis_test(RedisStorageMode::Hash).await else {
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

        let Some((_container, redis_adapter)) = setup_redis_test(RedisStorageMode::Hash).await else {
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
            docker_helpers::print_docker_unavailable_warning("Redis custom priority precedence test");
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
        let redis_adapter = RedisAdapter::with_priority(&url, "test_hash", RedisStorageMode::Hash, 3)
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
}
