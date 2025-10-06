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
}
