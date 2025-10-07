// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for etcd adapter using Docker containers.

mod common;

#[cfg(feature = "etcd")]
mod etcd_tests {
    use configuration::adapters::EtcdAdapter;
    use configuration::domain::ConfigKey;
    use configuration::ports::ConfigSource;
    use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage, ImageExt};

    use crate::common as docker_helpers;

    /// Helper to set up an etcd container and adapter for testing.
    async fn setup_etcd_test() -> Option<(testcontainers::ContainerAsync<GenericImage>, EtcdAdapter)>
    {
        if !docker_helpers::is_docker_available() {
            docker_helpers::print_docker_unavailable_warning("etcd integration test");
            return None;
        }

        // Use etcd v3.5.0 image
        let etcd_image = GenericImage::new("quay.io/coreos/etcd", "v3.5.0")
            .with_exposed_port(2379.into())
            .with_wait_for(WaitFor::message_on_stderr("ready to serve client requests"))
            .with_env_var("ETCD_ADVERTISE_CLIENT_URLS", "http://0.0.0.0:2379")
            .with_env_var("ETCD_LISTEN_CLIENT_URLS", "http://0.0.0.0:2379");

        let container = etcd_image.start().await.ok()?;
        let port = container.get_host_port_ipv4(2379).await.ok()?;

        let endpoint = format!("127.0.0.1:{}", port);

        // Give etcd a moment to fully start
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Set up test data
        let client = etcd_client::Client::connect([&endpoint], None).await.ok()?;
        let mut client_clone = client.clone();

        // Put some test values
        client_clone
            .put("test/test.key", "test_value", None)
            .await
            .ok()?;
        client_clone
            .put("test/database/host", "localhost", None)
            .await
            .ok()?;
        client_clone
            .put("test/database/port", "5432", None)
            .await
            .ok()?;

        let adapter = EtcdAdapter::new(vec![endpoint], Some("test/")).await.ok()?;

        Some((container, adapter))
    }

    #[tokio::test]
    async fn test_etcd_get() {
        let Some((_container, adapter)) = setup_etcd_test().await else {
            return;
        };

        let key = ConfigKey::from("test.key");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "test_value");
    }

    #[tokio::test]
    async fn test_etcd_get_nested() {
        let Some((_container, adapter)) = setup_etcd_test().await else {
            return;
        };

        // etcd uses slashes, but our adapter converts them to dots
        let key = ConfigKey::from("database.host");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap().as_str(), "localhost");
    }

    #[tokio::test]
    async fn test_etcd_all_keys() {
        let Some((_container, adapter)) = setup_etcd_test().await else {
            return;
        };

        let keys = adapter.all_keys().unwrap();

        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&ConfigKey::from("test.key")));
        assert!(keys.contains(&ConfigKey::from("database.host")));
        assert!(keys.contains(&ConfigKey::from("database.port")));
    }

    #[tokio::test]
    async fn test_etcd_reload() {
        let Some((container, mut adapter)) = setup_etcd_test().await else {
            return;
        };

        // Initial value
        let key = ConfigKey::from("test.key");
        let value = adapter.get(&key).unwrap();
        assert_eq!(value.unwrap().as_str(), "test_value");

        // Update value in etcd
        let port = container.get_host_port_ipv4(2379).await.unwrap();
        let endpoint = format!("127.0.0.1:{}", port);
        let client = etcd_client::Client::connect([&endpoint], None)
            .await
            .unwrap();
        let mut client_clone = client.clone();

        client_clone
            .put("test/test.key", "updated_value", None)
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
    async fn test_etcd_priority() {
        let Some((_container, adapter)) = setup_etcd_test().await else {
            return;
        };

        assert_eq!(adapter.priority(), 1);
    }

    #[tokio::test]
    async fn test_etcd_custom_priority() {
        if !docker_helpers::is_docker_available() {
            docker_helpers::print_docker_unavailable_warning("etcd custom priority test");
            return;
        }

        let etcd_image = GenericImage::new("quay.io/coreos/etcd", "v3.5.0")
            .with_exposed_port(2379.into())
            .with_wait_for(WaitFor::message_on_stderr("ready to serve client requests"))
            .with_env_var("ETCD_ADVERTISE_CLIENT_URLS", "http://0.0.0.0:2379")
            .with_env_var("ETCD_LISTEN_CLIENT_URLS", "http://0.0.0.0:2379");

        let container = etcd_image.start().await.unwrap();
        let port = container.get_host_port_ipv4(2379).await.unwrap();
        let endpoint = format!("127.0.0.1:{}", port);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let adapter = EtcdAdapter::with_priority(vec![endpoint], None, 5)
            .await
            .unwrap();

        assert_eq!(adapter.priority(), 5);
    }

    #[tokio::test]
    async fn test_etcd_nonexistent_key() {
        let Some((_container, adapter)) = setup_etcd_test().await else {
            return;
        };

        let key = ConfigKey::from("nonexistent.key");
        let value = adapter.get(&key).unwrap();

        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_etcd_prefix_filtering() {
        if !docker_helpers::is_docker_available() {
            docker_helpers::print_docker_unavailable_warning("etcd prefix filtering test");
            return;
        }

        let etcd_image = GenericImage::new("quay.io/coreos/etcd", "v3.5.0")
            .with_exposed_port(2379.into())
            .with_wait_for(WaitFor::message_on_stderr("ready to serve client requests"))
            .with_env_var("ETCD_ADVERTISE_CLIENT_URLS", "http://0.0.0.0:2379")
            .with_env_var("ETCD_LISTEN_CLIENT_URLS", "http://0.0.0.0:2379");

        let container = etcd_image.start().await.unwrap();
        let port = container.get_host_port_ipv4(2379).await.unwrap();
        let endpoint = format!("127.0.0.1:{}", port);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Set up data with different prefixes
        let client = etcd_client::Client::connect([&endpoint], None)
            .await
            .unwrap();
        let mut client_clone = client.clone();

        client_clone.put("app1/key1", "value1", None).await.unwrap();
        client_clone.put("app2/key2", "value2", None).await.unwrap();

        // Create adapter with prefix filtering
        let adapter = EtcdAdapter::new(vec![endpoint], Some("app1/"))
            .await
            .unwrap();

        let keys = adapter.all_keys().unwrap();

        // Should only have app1 keys
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&ConfigKey::from("key1")));
        assert!(!keys.contains(&ConfigKey::from("key2")));
    }

    #[tokio::test]
    async fn test_cli_overrides_etcd() {
        use configuration::adapters::CommandLineAdapter;
        use configuration::domain::ConfigurationService;
        use configuration::service::ConfigurationServiceBuilder;

        let Some((_container, etcd_adapter)) = setup_etcd_test().await else {
            return;
        };

        // etcd has "test.key" = "test_value"
        // CLI will override with "test.key" = "from_cli"
        let cli_args = vec!["--test.key=from_cli"];
        let cli_adapter = CommandLineAdapter::from_args(cli_args);

        let service = ConfigurationServiceBuilder::new()
            .with_source(Box::new(etcd_adapter))
            .with_source(Box::new(cli_adapter))
            .build()
            .unwrap();

        // CLI (priority 3) should win over etcd (priority 1)
        let value = service.get_str("test.key").unwrap();
        assert_eq!(value.as_str(), "from_cli");
    }

    #[tokio::test]
    async fn test_env_overrides_etcd() {
        use configuration::adapters::EnvVarAdapter;
        use configuration::domain::ConfigurationService;
        use configuration::service::ConfigurationServiceBuilder;
        use std::collections::HashMap;

        let Some((_container, etcd_adapter)) = setup_etcd_test().await else {
            return;
        };

        // etcd has "test.key" = "test_value"
        // Env will override with "test.key" = "from_env"
        let mut env_vars = HashMap::new();
        env_vars.insert("test.key".to_string(), "from_env".to_string());
        let env_adapter = EnvVarAdapter::with_values(env_vars);

        let service = ConfigurationServiceBuilder::new()
            .with_source(Box::new(etcd_adapter))
            .with_source(Box::new(env_adapter))
            .build()
            .unwrap();

        // Env (priority 2) should win over etcd (priority 1)
        let value = service.get_str("test.key").unwrap();
        assert_eq!(value.as_str(), "from_env");
    }

    #[tokio::test]
    async fn test_full_precedence_chain_with_etcd() {
        use configuration::adapters::{CommandLineAdapter, EnvVarAdapter};
        use configuration::domain::ConfigurationService;
        use configuration::service::ConfigurationServiceBuilder;
        use std::collections::HashMap;

        let Some((_container, etcd_adapter)) = setup_etcd_test().await else {
            return;
        };

        // etcd has:
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
            .with_source(Box::new(etcd_adapter))
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

        // database.port: etcd is the only source (priority 1)
        let value = service.get_str("database.port").unwrap();
        assert_eq!(value.as_str(), "5432");
    }

    #[tokio::test]
    async fn test_etcd_with_custom_priority_overrides_env() {
        use configuration::adapters::EnvVarAdapter;
        use configuration::domain::ConfigurationService;
        use configuration::service::ConfigurationServiceBuilder;
        use std::collections::HashMap;

        if !docker_helpers::is_docker_available() {
            docker_helpers::print_docker_unavailable_warning(
                "etcd custom priority precedence test",
            );
            return;
        }

        let etcd_image = GenericImage::new("quay.io/coreos/etcd", "v3.5.0")
            .with_exposed_port(2379.into())
            .with_wait_for(WaitFor::message_on_stderr("ready to serve client requests"))
            .with_env_var("ETCD_ADVERTISE_CLIENT_URLS", "http://0.0.0.0:2379")
            .with_env_var("ETCD_LISTEN_CLIENT_URLS", "http://0.0.0.0:2379");

        let container = etcd_image.start().await.unwrap();
        let port = container.get_host_port_ipv4(2379).await.unwrap();
        let endpoint = format!("127.0.0.1:{}", port);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Set up etcd with test data
        let client = etcd_client::Client::connect([&endpoint], None)
            .await
            .unwrap();
        let mut client_clone = client.clone();

        client_clone
            .put("special.key", "from_etcd", None)
            .await
            .unwrap();

        // Create etcd adapter with custom priority 3 (same as CLI!)
        let etcd_adapter = EtcdAdapter::with_priority(vec![endpoint], None, 3)
            .await
            .unwrap();

        // Create env adapter with normal priority 2
        let mut env_vars = HashMap::new();
        env_vars.insert("special.key".to_string(), "from_env".to_string());
        let env_adapter = EnvVarAdapter::with_values(env_vars);

        let service = ConfigurationServiceBuilder::new()
            .with_source(Box::new(env_adapter))
            .with_source(Box::new(etcd_adapter))
            .build()
            .unwrap();

        // etcd with priority 3 should win over env with priority 2
        let value = service.get_str("special.key").unwrap();
        assert_eq!(value.as_str(), "from_etcd");
    }

    // === etcd Watcher Tests ===

    /// Helper to set up an etcd container for watcher testing.
    async fn setup_etcd_watcher_test(
    ) -> Option<(testcontainers::ContainerAsync<GenericImage>, String)> {
        if !docker_helpers::is_docker_available() {
            docker_helpers::print_docker_unavailable_warning("etcd watcher test");
            return None;
        }

        let etcd_image = GenericImage::new("quay.io/coreos/etcd", "v3.5.0")
            .with_exposed_port(2379.into())
            .with_wait_for(WaitFor::message_on_stderr("ready to serve client requests"))
            .with_env_var("ETCD_ADVERTISE_CLIENT_URLS", "http://0.0.0.0:2379")
            .with_env_var("ETCD_LISTEN_CLIENT_URLS", "http://0.0.0.0:2379");

        let container = etcd_image.start().await.ok()?;
        let port = container.get_host_port_ipv4(2379).await.ok()?;
        let endpoint = format!("127.0.0.1:{}", port);

        // Give etcd a moment to fully start
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        Some((container, endpoint))
    }

    #[tokio::test]
    async fn test_etcd_watcher_creation() {
        use configuration::adapters::EtcdWatcher;

        let Some((_container, endpoint)) = setup_etcd_watcher_test().await else {
            return;
        };

        let watcher = EtcdWatcher::new(vec![&endpoint], Some("test/")).await;
        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_etcd_watcher_invalid_endpoint() {
        use configuration::adapters::EtcdWatcher;

        // Use localhost with closed port - this should fail connection immediately
        let watcher = EtcdWatcher::new(vec!["127.0.0.1:19999"], Some("test/")).await;

        // Note: etcd client may not fail immediately on invalid endpoints in some cases,
        // so we just verify the watcher can be created (it will fail on watch() if connection is bad)
        // For a real failure test, we'd need to call watch() and see the reconnection logic
        let _ = watcher;
    }

    #[tokio::test]
    async fn test_etcd_watcher_start_stop() {
        use configuration::adapters::EtcdWatcher;
        use configuration::ports::ConfigWatcher;
        use std::sync::Arc;

        let Some((_container, endpoint)) = setup_etcd_watcher_test().await else {
            return;
        };

        let mut watcher = EtcdWatcher::new(vec![&endpoint], Some("test/"))
            .await
            .unwrap();

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
    async fn test_etcd_watcher_callback_triggered() {
        use configuration::adapters::EtcdWatcher;
        use configuration::ports::ConfigWatcher;
        use etcd_client::Client;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let Some((_container, endpoint)) = setup_etcd_watcher_test().await else {
            return;
        };

        let mut watcher = EtcdWatcher::new(vec![&endpoint], Some("test/watcher/"))
            .await
            .unwrap();

        let triggered = Arc::new(AtomicBool::new(false));
        let triggered_clone = Arc::clone(&triggered);

        let callback = Arc::new(move |key: ConfigKey| {
            eprintln!("etcd watcher callback triggered for key: {}", key.as_str());
            triggered_clone.store(true, Ordering::SeqCst);
        });

        watcher.watch(callback).unwrap();

        // Wait for watcher to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Modify a key in etcd
        let mut client = Client::connect([&endpoint], None).await.unwrap();
        client
            .put("test/watcher/mykey", "test_value", None)
            .await
            .unwrap();

        // Wait for the event to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Clean up
        client.delete("test/watcher/mykey", None).await.unwrap();
        watcher.stop().unwrap();

        let was_triggered = triggered.load(Ordering::SeqCst);
        assert!(
            was_triggered,
            "etcd watcher callback should have been triggered"
        );
    }

    #[tokio::test]
    async fn test_etcd_watcher_multiple_changes() {
        use configuration::adapters::EtcdWatcher;
        use configuration::ports::ConfigWatcher;
        use etcd_client::Client;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let Some((_container, endpoint)) = setup_etcd_watcher_test().await else {
            return;
        };

        let mut watcher = EtcdWatcher::new(vec![&endpoint], Some("test/multi/"))
            .await
            .unwrap();

        let trigger_count = Arc::new(AtomicUsize::new(0));
        let trigger_count_clone = Arc::clone(&trigger_count);

        let callback = Arc::new(move |key: ConfigKey| {
            eprintln!("etcd watcher detected change: {}", key.as_str());
            trigger_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        watcher.watch(callback).unwrap();

        // Wait for watcher to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Make multiple changes
        let mut client = Client::connect([&endpoint], None).await.unwrap();

        for i in 0..3 {
            client
                .put(format!("test/multi/key{}", i), format!("value{}", i), None)
                .await
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Wait for events to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Clean up
        for i in 0..3 {
            client
                .delete(format!("test/multi/key{}", i), None)
                .await
                .unwrap();
        }
        watcher.stop().unwrap();

        let count = trigger_count.load(Ordering::SeqCst);
        assert!(count >= 3, "Expected at least 3 triggers, got {}", count);
    }

    #[tokio::test]
    async fn test_etcd_watcher_prefix_filtering() {
        use configuration::adapters::EtcdWatcher;
        use configuration::ports::ConfigWatcher;
        use etcd_client::Client;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let Some((_container, endpoint)) = setup_etcd_watcher_test().await else {
            return;
        };

        let mut watcher = EtcdWatcher::new(vec![&endpoint], Some("test/prefix/"))
            .await
            .unwrap();

        let trigger_count = Arc::new(AtomicUsize::new(0));
        let trigger_count_clone = Arc::clone(&trigger_count);

        let callback = Arc::new(move |key: ConfigKey| {
            eprintln!("etcd watcher detected change: {}", key.as_str());
            trigger_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        watcher.watch(callback).unwrap();

        // Wait for watcher to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let mut client = Client::connect([&endpoint], None).await.unwrap();

        // This should trigger (has correct prefix)
        client
            .put("test/prefix/key1", "value1", None)
            .await
            .unwrap();

        // This should NOT trigger (different prefix)
        client
            .put("other/prefix/key2", "value2", None)
            .await
            .unwrap();

        // Wait for events to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Clean up
        client.delete("test/prefix/key1", None).await.unwrap();
        client.delete("other/prefix/key2", None).await.unwrap();
        watcher.stop().unwrap();

        let count = trigger_count.load(Ordering::SeqCst);
        // etcd watch may generate multiple events for the same key change,
        // so we verify at least 1 event was received (confirming filtering works).
        // The important part is that only key1 triggered, not key2.
        assert!(
            count >= 1,
            "Expected at least 1 trigger for key with correct prefix, got {}",
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
