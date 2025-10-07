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
        let client = etcd_client::Client::connect([&endpoint], None)
            .await
            .ok()?;
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

        let adapter = EtcdAdapter::new(vec![endpoint], Some("test/"))
            .await
            .ok()?;

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
            docker_helpers::print_docker_unavailable_warning("etcd custom priority precedence test");
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
}
