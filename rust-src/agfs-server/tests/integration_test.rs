//! AGFS Integration Tests
//!
//! End-to-end tests for the AGFS HTTP API.
//!
//! Run with: cargo test --package agfs-server --test integration_test -- --ignored -- --test-threads=1
//!
//! Prerequisites:
//! - Server running on http://127.0.0.1:8080
//! - Or start with: cargo run --release --bin agfs-server -- --config test-config.yaml

use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;

const SERVER_URL: &str = "http://127.0.0.1:8080";

/// AGFS API client for integration tests
struct AgfsClient {
    client: Client,
    base_url: String,
}

impl AgfsClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: SERVER_URL.to_string(),
        }
    }

    #[allow(dead_code)]
    fn new_with_url(url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: url.to_string(),
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Box<dyn std::error::Error>> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }

    #[allow(dead_code)]
    async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.post(&url).json(body).send().await?;
        self.handle_response(response).await
    }

    #[allow(dead_code)]
    async fn put<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.put(&url).json(body).send().await?;
        self.handle_response(response).await
    }

    #[allow(dead_code)]
    async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T, Box<dyn std::error::Error>> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.delete(&url).send().await?;
        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(format!("Request failed with status {}: {}", status, text).into());
        }

        serde_json::from_str(&text).map_err(|e| e.into())
    }

    // Specific API methods

    async fn health(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;
        Ok(serde_json::from_str(&text)?)
    }

    async fn capabilities(&self) -> Result<Value, Box<dyn std::error::Error>> {
        self.get("/api/v1/capabilities").await
    }

    async fn stat(&self, path: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/stat?path={}", self.base_url, path);
        let response = self.client.get(&url).send().await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(format!("Stat failed: {}", text).into());
        }
        Ok(serde_json::from_str(&text)?)
    }

    async fn read_dir(&self, path: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/directories?path={}", self.base_url, path);
        let response = self.client.get(&url).send().await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(format!("ReadDir failed: {}", text).into());
        }
        Ok(serde_json::from_str(&text)?)
    }

    async fn read(&self, path: &str, offset: i64, size: i64) -> Result<bytes::Bytes, Box<dyn std::error::Error>> {
        let url = format!(
            "{}{}?path={}&offset={}&size={}",
            self.base_url, "/api/v1/files", path, offset, size
        );
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(format!("Read failed: {}", response.status()).into());
        }
        Ok(response.bytes().await?)
    }

    async fn write(&self, path: &str, data: &[u8]) -> Result<Value, Box<dyn std::error::Error>> {
        // Use PUT to write, path is query parameter
        let url = format!("{}{}?path={}", self.base_url, "/api/v1/files", path);
        let response = self.client.put(&url).body(data.to_vec()).send().await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(format!("Write failed: {}", text).into());
        }
        Ok(serde_json::from_str(&text)?)
    }

    async fn create(&self, path: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}{}?path={}", self.base_url, "/api/v1/files", path);
        let response = self.client.post(&url).send().await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(format!("Create failed: {}", text).into());
        }
        Ok(serde_json::from_str(&text)?)
    }

    async fn mkdir(&self, path: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}{}?path={}", self.base_url, "/api/v1/directories", path);
        let response = self.client.post(&url).send().await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(format!("Mkdir failed: {}", text).into());
        }
        Ok(serde_json::from_str(&text)?)
    }

    async fn remove(&self, path: &str) -> Result<Value, Box<dyn std::error::Error>> {
        // DELETE is actually POST to /api/v1/files/delete
        let url = format!("{}{}?path={}", self.base_url, "/api/v1/files/delete", path);
        let response = self.client.post(&url).send().await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(format!("Remove failed: {}", text).into());
        }
        Ok(serde_json::from_str(&text)?)
    }

    async fn plugins(&self) -> Result<Value, Box<dyn std::error::Error>> {
        self.get("/api/v1/plugins").await
    }
}

impl Default for AgfsClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_health_check() {
    let client = AgfsClient::new();
    let result = client.health().await.unwrap();
    assert_eq!(result["status"], "healthy");
}

#[tokio::test]
#[ignore]
async fn test_capabilities() {
    let client = AgfsClient::new();
    let result = client.capabilities().await.unwrap();
    // Capabilities endpoint returns features array or plugins array
    assert!(result["features"].is_array() || result["plugins"].is_array() || result["capabilities"].is_array());
}

#[tokio::test]
#[ignore]
async fn test_plugins_list() {
    let client = AgfsClient::new();
    let result = client.plugins().await.unwrap();
    let plugins = result["plugins"].as_array().unwrap();
    assert!(!plugins.is_empty());

    // Check for expected plugins
    let plugin_names: Vec<&str> = plugins
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();

    assert!(plugin_names.contains(&"memfs"));
    assert!(plugin_names.contains(&"localfs"));
}

#[tokio::test]
#[ignore]
async fn test_memfs_basic_operations() {
    let client = AgfsClient::new();

    // Create a file
    let test_path = "/memfs/test-integration.txt";
    let test_content = b"Hello from integration test!";

    // Cleanup first in case it exists from previous run
    let _ = client.remove(test_path).await;

    // Create the file
    let create_result = client.create(test_path).await.unwrap();
    assert_eq!(create_result["message"], "file created");

    // Write data
    let write_result = client.write(test_path, test_content).await.unwrap();
    assert!(write_result["message"].as_str().unwrap().contains("bytes"));

    // Stat the file
    let stat_result = client.stat(test_path).await.unwrap();
    assert_eq!(stat_result["name"], "test-integration.txt");
    assert!(stat_result["size"].as_i64().unwrap() > 0);

    // Read the file
    let read_data = client.read(test_path, 0, 100).await.unwrap();
    assert_eq!(read_data.as_ref(), test_content);

    // Read directory - response format is {"files": [...]}
    let dir_result = client.read_dir("/memfs").await.unwrap();
    let files = dir_result["files"].as_array().unwrap();
    assert!(files.iter().any(|e| e["name"] == "test-integration.txt"));

    // Cleanup
    client.remove(test_path).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_memfs_directory_operations() {
    let client = AgfsClient::new();

    let test_dir = "/memfs/test-dir";
    let test_file = format!("{}/nested.txt", test_dir);

    // Cleanup first in case it exists from previous run
    let _ = client.remove(&test_file).await;
    let _ = client.remove(test_dir).await;

    // Create directory
    client.mkdir(test_dir).await.unwrap();

    // Verify directory exists
    let stat_result = client.stat(test_dir).await.unwrap();
    assert!(stat_result["isDir"].as_bool().unwrap());

    // Create file in directory
    client.create(&test_file).await.unwrap();
    client.write(&test_file, b"nested content").await.unwrap();

    // Read directory - response format is {"files": [...]}
    let dir_result = client.read_dir(test_dir).await.unwrap();
    let files = dir_result["files"].as_array().unwrap();
    assert!(files.iter().any(|e| e["name"] == "nested.txt"));

    // Cleanup
    client.remove(&test_file).await.unwrap();
    client.remove(test_dir).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_localfs_operations() {
    let client = AgfsClient::new();

    // Create a file in localfs (which maps to /tmp/agfs-local)
    let test_path = "/local/test-integration.txt";
    let test_content = b"LocalFS test content";

    // Cleanup first in case it exists from previous run
    let _ = client.remove(test_path).await;
    let _ = std::fs::remove_file("/tmp/agfs-local/test-integration.txt");

    client.create(test_path).await.unwrap();
    client.write(test_path, test_content).await.unwrap();

    // Verify it exists
    let stat_result = client.stat(test_path).await.unwrap();
    assert_eq!(stat_result["name"], "test-integration.txt");

    // Read it back
    let read_data = client.read(test_path, 0, 100).await.unwrap();
    assert_eq!(read_data.as_ref(), test_content);

    // Verify file actually exists on disk
    assert!(std::path::Path::new("/tmp/agfs-local/test-integration.txt").exists());

    // Cleanup
    client.remove(test_path).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_offset_read() {
    let client = AgfsClient::new();

    let test_path = "/memfs/test-offset.txt";
    let test_content = b"0123456789ABCDEFGHIJ";

    // Cleanup first in case it exists from previous run
    let _ = client.remove(test_path).await;

    client.create(test_path).await.unwrap();
    client.write(test_path, test_content).await.unwrap();

    // Read from offset 10
    let partial_data = client.read(test_path, 10, 10).await.unwrap();
    assert_eq!(partial_data.as_ref(), b"ABCDEFGHIJ");

    // Cleanup
    client.remove(test_path).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_nonexistent_path() {
    let client = AgfsClient::new();

    // Stat on nonexistent path should return error
    let result = client.stat("/memfs/this-does-not-exist.txt").await;
    assert!(result.is_err());

    // Read nonexistent file should error
    let result = client.read("/memfs/also-does-not-exist.txt", 0, 100).await;
    assert!(result.is_err());
}
