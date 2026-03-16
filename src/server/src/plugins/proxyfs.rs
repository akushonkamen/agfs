//! ProxyFS - Proxy to Remote AGFS Server
//!
//! Forwards all file system operations to a remote AGFS HTTP API server.

use ctxfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Simple URL encoding function
fn encode_url_path(path: &str) -> String {
    path.chars().map(|c| match c {
        'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' | '/' => c.to_string(),
        _ => format!("%{:02X}", c as u8),
    }).collect()
}

/// HTTP client for communicating with remote AGFS server
#[derive(Debug, Clone)]
struct AgfsClient {
    base_url: String,
    client: reqwest::Client,
}

impl AgfsClient {
    /// Create a new AGFS client
    fn new(base_url: impl Into<String>) -> Result<Self, AgfsError> {
        let base_url = base_url.into();
        // Validate URL format
        if !base_url.contains("://") {
            return Err(AgfsError::invalid_argument(
                format!("invalid base_url format: {} (expected format: http://hostname:port)", base_url)
            ));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AgfsError::internal(format!("failed to create HTTP client: {}", e)))?;

        Ok(Self { base_url, client })
    }

    /// Health check
    async fn health(&self) -> Result<(), AgfsError> {
        let url = format!("{}/health", self.base_url.trim_end_matches('/'));
        self.client.get(&url)
            .send()
            .await
            .map_err(|e| AgfsError::internal(format!("health check failed: {}", e)))?
            .error_for_status()
            .map_err(|e| AgfsError::internal(format!("health check error: {}", e)))?;
        Ok(())
    }

    /// Read file from remote
    async fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let url = format!("{}/files?path={}", self.base_url.trim_end_matches('/'), encode_url_path(path));
        let resp = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AgfsError::internal(format!("read request failed: {}", e)))?;

        if resp.status().is_success() {
            let data = resp.bytes().await
                .map_err(|e| AgfsError::internal(format!("failed to read response: {}", e)))?
                .to_vec();

            let offset = if offset < 0 { 0 } else { offset as usize };
            let size = if size < 0 { data.len() - offset } else { size as usize };

            if offset >= data.len() {
                return Ok(Vec::new());
            }

            let end = (offset + size).min(data.len());
            Ok(data[offset..end].to_vec())
        } else {
            Err(AgfsError::internal(format!("read failed: {}", resp.status())))
        }
    }

    /// Write file to remote
    async fn write(&self, path: &str, data: &[u8]) -> Result<(), AgfsError> {
        let url = format!("{}/files?path={}", self.base_url.trim_end_matches('/'), encode_url_path(path));
        let resp = self.client.post(&url)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| AgfsError::internal(format!("write request failed: {}", e)))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AgfsError::internal(format!("write failed: {}", resp.status())))
        }
    }

    /// Read directory from remote
    async fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        let url = format!("{}/directories?path={}", self.base_url.trim_end_matches('/'), encode_url_path(path));
        let resp = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AgfsError::internal(format!("read_dir request failed: {}", e)))?;

        if resp.status().is_success() {
            #[derive(serde::Deserialize)]
            struct FilesResponse {
                files: Vec<RemoteFileInfo>,
            }

            #[derive(serde::Deserialize)]
            struct RemoteFileInfo {
                name: String,
                size: i64,
                mode: u32,
                mod_time: chrono::DateTime<chrono::Utc>,
                is_dir: bool,
            }

            let resp_json: FilesResponse = resp.json().await
                .map_err(|e| AgfsError::internal(format!("failed to parse response: {}", e)))?;

            Ok(resp_json.files.into_iter().map(|f| FileInfo {
                name: f.name,
                size: f.size,
                mode: f.mode,
                mod_time: f.mod_time,
                is_dir: f.is_dir,
                is_symlink: false,
                meta: MetaData::default(),
            }).collect())
        } else {
            Err(AgfsError::internal(format!("read_dir failed: {}", resp.status())))
        }
    }

    /// Stat file from remote
    async fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        let url = format!("{}/stat?path={}", self.base_url.trim_end_matches('/'), encode_url_path(path));
        let resp = self.client.get(&url)
            .send()
            .await
            .map_err(|e| AgfsError::internal(format!("stat request failed: {}", e)))?;

        if resp.status().is_success() {
            #[derive(serde::Deserialize)]
            struct StatResponse {
                name: String,
                size: i64,
                mode: u32,
                mod_time: chrono::DateTime<chrono::Utc>,
                is_dir: bool,
            }

            let resp_json: StatResponse = resp.json().await
                .map_err(|e| AgfsError::internal(format!("failed to parse response: {}", e)))?;

            Ok(FileInfo {
                name: resp_json.name,
                size: resp_json.size,
                mode: resp_json.mode,
                mod_time: resp_json.mod_time,
                is_dir: resp_json.is_dir,
                is_symlink: false,
                meta: MetaData::default(),
            })
        } else {
            Err(AgfsError::internal(format!("stat failed: {}", resp.status())))
        }
    }

    /// Create file on remote
    async fn create(&self, path: &str) -> Result<(), AgfsError> {
        let url = format!("{}/files?path={}", self.base_url.trim_end_matches('/'), encode_url_path(path));
        let resp = self.client.put(&url)
            .send()
            .await
            .map_err(|e| AgfsError::internal(format!("create request failed: {}", e)))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AgfsError::internal(format!("create failed: {}", resp.status())))
        }
    }

    /// Remove file from remote
    async fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let url = format!("{}/files?path={}", self.base_url.trim_end_matches('/'), encode_url_path(path));
        let resp = self.client.delete(&url)
            .send()
            .await
            .map_err(|e| AgfsError::internal(format!("remove request failed: {}", e)))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AgfsError::internal(format!("remove failed: {}", resp.status())))
        }
    }
}

/// ProxyFS - Proxies all operations to a remote AGFS server
#[derive(Debug, Clone)]
pub struct ProxyFS {
    client: Arc<RwLock<Option<AgfsClient>>>,
    base_url: String,
    plugin_name: String,
}

impl ProxyFS {
    /// Create a new ProxyFS instance
    pub fn new(base_url: impl Into<String>, plugin_name: impl Into<String>) -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            base_url: base_url.into(),
            plugin_name: plugin_name.into(),
        }
    }

    /// Reload the proxy connection
    pub async fn reload(&self) -> Result<(), AgfsError> {
        let new_client = AgfsClient::new(&self.base_url)?;
        new_client.health().await?;
        *self.client.write().await = Some(new_client);
        Ok(())
    }

    /// Get or create the client
    async fn get_client(&self) -> Result<AgfsClient, AgfsError> {
        let client_read = self.client.read().await;
        if let Some(ref client) = *client_read {
            Ok(client.clone())
        } else {
            drop(client_read);
            self.reload().await?;
            let client_read = self.client.read().await;
            client_read.as_ref().cloned()
                .ok_or_else(|| AgfsError::internal("failed to initialize client".to_string()))
        }
    }

    /// Blocking version of get_client for sync trait methods
    #[allow(dead_code)]
    fn get_client_blocking(&self) -> Result<AgfsClient, AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;
        runtime.block_on(self.get_client())
    }
}

impl FileSystem for ProxyFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        if path == "/reload" {
            return Ok(()); // Virtual file
        }
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;
        runtime.block_on(async {
            let client = self.get_client().await?;
            client.create(path).await
        })
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        // Proxy to remote - not implemented in basic version
        Err(AgfsError::NotSupported)
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;
        runtime.block_on(async {
            let client = self.get_client().await?;
            client.remove(path).await
        })
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        self.remove(path)
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        // Special handling for /reload
        if path == "/reload" {
            let data = b"Write to this file to reload the proxy connection\n";
            let offset = if offset < 0 { 0 } else { offset as usize };
            let size = if size < 0 { data.len() - offset } else { size as usize };
            if offset >= data.len() {
                return Ok(Vec::new());
            }
            let end = (offset + size).min(data.len());
            return Ok(data[offset..end].to_vec());
        }

        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;
        runtime.block_on(async {
            let client = self.get_client().await?;
            client.read(path, offset, size).await
        })
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, _flags: WriteFlag) -> Result<i64, AgfsError> {
        // Special handling for /reload - trigger hot reload
        if path == "/reload" {
            let runtime = tokio::runtime::Handle::try_current()
                .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;
            runtime.block_on(async {
                self.reload().await
            })?;
            return Ok(data.len() as i64);
        }

        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;
        runtime.block_on(async {
            let client = self.get_client().await?;
            client.write(path, data).await?;
            Ok(data.len() as i64)
        })
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;
        let mut files = runtime.block_on(async {
            let client = self.get_client().await?;
            client.read_dir(path).await
        })?;

        // Add /reload virtual file to root directory listing
        if path == "/" || path.is_empty() {
            files.push(FileInfo {
                name: "reload".to_string(),
                size: 0,
                mode: 0o200, // write-only
                mod_time: Utc::now(),
                is_dir: false,
                is_symlink: false,
                meta: MetaData {
                    name: self.plugin_name.clone(),
                    r#type: "control".to_string(),
                    content: {
                        let mut map = HashMap::new();
                        map.insert("description".to_string(), "Write to this file to reload proxy connection".to_string());
                        map.insert("remote-url".to_string(), self.base_url.clone());
                        map
                    },
                },
            });
        }

        Ok(files)
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        // Special handling for /reload
        if path == "/reload" {
            return Ok(FileInfo {
                name: "reload".to_string(),
                size: 0,
                mode: 0o200, // write-only
                mod_time: Utc::now(),
                is_dir: false,
                is_symlink: false,
                meta: MetaData {
                    name: self.plugin_name.clone(),
                    r#type: "control".to_string(),
                    content: {
                        let mut map = HashMap::new();
                        map.insert("description".to_string(), "Write to this file to reload proxy connection".to_string());
                        map.insert("remote-url".to_string(), self.base_url.clone());
                        map
                    },
                },
            });
        }

        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;
        runtime.block_on(async {
            let client = self.get_client().await?;
            client.stat(path).await
        })
    }

    fn rename(&self, _old_path: &str, _new_path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn chmod(&self, _path: &str, _mode: u32) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn open(&self, path: &str) -> Result<Box<dyn std::io::Read + Send>, AgfsError> {
        let data = self.read(path, 0, -1)?;
        Ok(Box::new(std::io::Cursor::new(data)))
    }

    fn open_write(&self, _path: &str) -> Result<Box<dyn std::io::Write + Send>, AgfsError> {
        Err(AgfsError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxyfs_invalid_url() {
        let fs = ProxyFS::new("invalid-url", "test");
        assert!(fs.client.blocking_read().is_none());
    }

    #[test]
    fn test_proxyfs_create() {
        let fs = ProxyFS::new("http://localhost:8080/api/v1", "test");
        // Cannot test further without a running server
        assert_eq!(fs.base_url, "http://localhost:8080/api/v1");
    }
}
