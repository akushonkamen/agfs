//! AGFS HTTP client
//!
//! This module provides the HTTP client for interacting with AGFS server.
//! Based on the Go implementation in `agfs-sdk/go/client.go`.

use crate::error::AgfsError;
use crate::types::{
    CapabilitiesResponse, ChmodRequest, DigestRequest, DigestResponse, ErrorResponse,
    FileInfo, GrepRequest, GrepResponse, HandleInfo, HandleResponse, ListResponse,
    OpenFlag, ReadlinkResponse, RenameRequest, SuccessResponse, SymlinkRequest,
    WriteFlag,
};
use reqwest::{Client as HttpClient, Response};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// AGFS client for connecting to AGFS server
///
/// This client provides methods to interact with all AGFS server API endpoints.
///
/// # Example
///
/// ```no_run
/// use agfs_sdk::Client;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new("http://localhost:8080")?;
///
/// // Create a file
/// client.create("/test/file.txt").await?;
///
/// // Write data
/// client.write("/test/file.txt", b"Hello, World!").await?;
///
/// // Read data
/// let data = client.read("/test/file.txt", 0, -1).await?;
/// # Ok(())
/// # }
/// ```
pub struct Client {
    /// Base URL for API requests (includes /api/v1)
    base_url: String,

    /// HTTP client
    http_client: Arc<HttpClient>,
}

impl Client {
    /// Create a new AGFS client
    ///
    /// The base_url can be either a full URL with "/api/v1" or just the base.
    /// If "/api/v1" is not present, it will be automatically appended.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use agfs_sdk::Client;
    ///
    /// let client = Client::new("http://localhost:8080").unwrap();
    /// // or
    /// let client = Client::new("http://localhost:8080/api/v1").unwrap();
    /// ```
    pub fn new(base_url: impl Into<String>) -> Result<Self, reqwest::Error> {
        let base_url = normalize_base_url(base_url.into());
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            base_url,
            http_client: Arc::new(http_client),
        })
    }

    /// Create a new AGFS client with a custom HTTP client
    pub fn new_with_http_client(
        base_url: impl Into<String>,
        http_client: HttpClient,
    ) -> Self {
        let base_url = normalize_base_url(base_url.into());
        Self {
            base_url,
            http_client: Arc::new(http_client),
        }
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Health check
    ///
    /// Checks if the AGFS server is healthy and responding.
    pub async fn health(&self) -> Result<(), AgfsError> {
        let url = format!("{}/health", self.base_url);
        let response = self.http_client.get(&url).send().await?;
        self.handle_response::<()>(response).await?;
        Ok(())
    }

    /// Get server capabilities
    ///
    /// Returns information about the server version and supported features.
    pub async fn get_capabilities(&self) -> Result<CapabilitiesResponse, AgfsError> {
        let url = format!("{}/capabilities", self.base_url);
        let response = self.http_client.get(&url).send().await?;

        if response.status() == 404 {
            // Older servers without this endpoint
            return Ok(CapabilitiesResponse {
                version: "unknown".to_string(),
                features: vec![],
            });
        }

        self.handle_response(response).await
    }

    /// Create a new file
    ///
    /// Creates a new empty file at the specified path.
    pub async fn create(&self, path: &str) -> Result<(), AgfsError> {
        let url = format!("{}/files?path={}", self.base_url, encode_path(path));
        let response = self.http_client.post(&url).send().await?;
        self.handle_response(response).await
    }

    /// Read file content
    ///
    /// Reads up to `size` bytes from the file starting at `offset`.
    ///
    /// # Arguments
    /// - `path`: The file path to read from
    /// - `offset`: Starting position in bytes (0 for beginning)
    /// - `size`: Number of bytes to read (-1 means read all)
    pub async fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let mut url = format!("{}/files?path={}", self.base_url, encode_path(path));

        if offset > 0 {
            url.push_str(&format!("&offset={}", offset));
        }
        if size >= 0 {
            url.push_str(&format!("&size={}", size));
        }

        let response = self.http_client.get(&url).send().await?;
        let bytes = self.handle_response_bytes(response).await?;
        Ok(bytes)
    }

    /// Write data to a file
    ///
    /// Writes data to a file at the specified path.
    /// Creates the file if it doesn't exist.
    pub async fn write(&self, path: &str, data: &[u8]) -> Result<Vec<u8>, AgfsError> {
        self.write_with_flags(path, data, 0, WriteFlag::CREATE | WriteFlag::TRUNCATE)
            .await
    }

    /// Write data with flags
    ///
    /// Writes data to a file with offset and flags controlling behavior.
    pub async fn write_with_flags(
        &self,
        path: &str,
        data: &[u8],
        offset: i64,
        flags: WriteFlag,
    ) -> Result<Vec<u8>, AgfsError> {
        let url = format!(
            "{}/files?path={}&offset={}&flags={}",
            self.base_url,
            encode_path(path),
            offset,
            flags.bits()
        );

        let response = self
            .http_client
            .put(&url)
            .body(data.to_vec())
            .send()
            .await?;

        let resp: SuccessResponse = self.handle_response(response).await?;
        Ok(resp.message.into_bytes())
    }

    /// Delete a file or directory
    ///
    /// Removes the file or directory at the specified path.
    ///
    /// # Arguments
    /// - `path`: The path to remove
    /// - `recursive`: If true, removes non-empty directories recursively
    pub async fn remove(&self, path: &str, recursive: bool) -> Result<(), AgfsError> {
        let url = format!(
            "{}/files?path={}&recursive={}",
            self.base_url,
            encode_path(path),
            recursive
        );

        let response = self.http_client.delete(&url).send().await?;
        self.handle_response(response).await
    }

    /// Remove a single file or empty directory
    ///
    /// Convenience method that calls `remove` with `recursive=false`.
    pub async fn remove_one(&self, path: &str) -> Result<(), AgfsError> {
        self.remove(path, false).await
    }

    /// Remove a path recursively
    ///
    /// Convenience method that calls `remove` with `recursive=true`.
    pub async fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        self.remove(path, true).await
    }

    /// Create a directory
    ///
    /// Creates a new directory at the specified path with the given permissions.
    pub async fn mkdir(&self, path: &str, perm: u32) -> Result<(), AgfsError> {
        let url = format!(
            "{}/directories?path={}&mode={:o}",
            self.base_url,
            encode_path(path),
            perm
        );

        let response = self.http_client.post(&url).send().await?;
        self.handle_response(response).await
    }

    /// List directory contents
    ///
    /// Returns a list of file info entries for the directory at the specified path.
    pub async fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        let url = format!("{}/directories?path={}", self.base_url, encode_path(path));
        let response = self.http_client.get(&url).send().await?;

        let list: ListResponse = self.handle_response(response).await?;
        Ok(list.files)
    }

    /// Get file information
    ///
    /// Returns metadata about the file or directory at the specified path.
    pub async fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        let url = format!("{}/stat?path={}", self.base_url, encode_path(path));
        let response = self.http_client.get(&url).send().await?;

        self.handle_response(response).await
    }

    /// Rename/move a file or directory
    ///
    /// Moves a file or directory from `old_path` to `new_path`.
    pub async fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError> {
        let url = format!("{}/rename?path={}", self.base_url, encode_path(old_path));

        let body = serde_json::to_string(&RenameRequest {
            new_path: new_path.to_string(),
        })?;

        let response = self
            .http_client
            .post(&url)
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Change file permissions
    ///
    /// Changes the permission bits of the file at the specified path.
    pub async fn chmod(&self, path: &str, mode: u32) -> Result<(), AgfsError> {
        let url = format!("{}/chmod?path={}", self.base_url, encode_path(path));

        let body = serde_json::to_string(&ChmodRequest { mode })?;

        let response = self
            .http_client
            .post(&url)
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Update file modification time
    ///
    /// Touches the file at the specified path, creating it if it doesn't exist.
    pub async fn touch(&self, path: &str) -> Result<(), AgfsError> {
        let url = format!("{}/touch?path={}", self.base_url, encode_path(path));
        let response = self.http_client.post(&url).send().await?;
        self.handle_response(response).await
    }

    /// Truncate a file
    ///
    /// Truncates the file at the specified path to the given size.
    pub async fn truncate(&self, path: &str, size: i64) -> Result<(), AgfsError> {
        let url = format!(
            "{}/truncate?path={}&size={}",
            self.base_url,
            encode_path(path),
            size
        );

        let response = self.http_client.post(&url).send().await?;
        self.handle_response(response).await
    }

    /// Create a symbolic link
    ///
    /// Creates a symbolic link at `link_path` pointing to `target_path`.
    pub async fn symlink(&self, target: &str, link: &str) -> Result<(), AgfsError> {
        let url = format!("{}/symlink?path={}", self.base_url, encode_path(link));

        let body = serde_json::to_string(&SymlinkRequest {
            target: target.to_string(),
        })?;

        let response = self
            .http_client
            .post(&url)
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Read symbolic link target
    ///
    /// Returns the target path that the symbolic link at `link_path` points to.
    pub async fn readlink(&self, link: &str) -> Result<String, AgfsError> {
        let url = format!("{}/readlink?path={}", self.base_url, encode_path(link));
        let response = self.http_client.get(&url).send().await?;

        let resp: ReadlinkResponse = self.handle_response(response).await?;
        Ok(resp.target)
    }

    /// Search for a pattern in files
    ///
    /// Performs a regex search for `pattern` in files under `path`.
    ///
    /// # Arguments
    /// - `path`: Directory path to search in
    /// - `pattern`: Regular expression pattern to search for
    /// - `recursive`: Whether to search recursively in subdirectories
    /// - `case_insensitive`: Whether to perform case-insensitive matching
    pub async fn grep(
        &self,
        path: &str,
        pattern: &str,
        recursive: bool,
        case_insensitive: bool,
    ) -> Result<GrepResponse, AgfsError> {
        let url = format!("{}/grep", self.base_url);

        let body = serde_json::to_string(&GrepRequest {
            path: path.to_string(),
            pattern: pattern.to_string(),
            recursive,
            case_insensitive,
        })?;

        let response = self
            .http_client
            .post(&url)
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Calculate file digest
    ///
    /// Calculates the hash digest of a file using the specified algorithm.
    ///
    /// # Arguments
    /// - `path`: File path
    /// - `algorithm`: Hash algorithm ("xxh3" or "md5")
    pub async fn digest(&self, path: &str, algorithm: &str) -> Result<DigestResponse, AgfsError> {
        let url = format!("{}/digest", self.base_url);

        let body = serde_json::to_string(&DigestRequest {
            algorithm: algorithm.to_string(),
            path: path.to_string(),
        })?;

        let response = self
            .http_client
            .post(&url)
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        self.handle_response(response).await
    }

    // --- Handle operations ---

    /// Open a file handle
    ///
    /// Opens a file and returns a handle ID for subsequent operations.
    pub async fn open_handle(&self, path: &str, flags: OpenFlag, mode: u32) -> Result<i64, AgfsError> {
        let url = format!(
            "{}/handles/open?path={}&flags={}&mode={:o}",
            self.base_url,
            encode_path(path),
            flags.bits(),
            mode
        );

        let response = self.http_client.post(&url).send().await?;

        if response.status() == 501 {
            return Err(AgfsError::NotSupported);
        }

        let resp: HandleResponse = self.handle_response(response).await?;
        Ok(resp.handle_id)
    }

    /// Close a file handle
    ///
    /// Closes the file handle with the given ID.
    pub async fn close_handle(&self, handle_id: i64) -> Result<(), AgfsError> {
        let url = format!("{}/handles/{}", self.base_url, handle_id);
        let response = self.http_client.delete(&url).send().await?;
        self.handle_response(response).await
    }

    /// Read from a file handle
    ///
    /// Reads up to `size` bytes from the file handle starting at `offset`.
    pub async fn read_handle(&self, handle_id: i64, offset: i64, size: i32) -> Result<Vec<u8>, AgfsError> {
        let url = format!(
            "{}/handles/{}/read?offset={}&size={}",
            self.base_url, handle_id, offset, size
        );

        let response = self.http_client.get(&url).send().await?;
        self.handle_response_bytes(response).await
    }

    /// Write to a file handle
    ///
    /// Writes data to the file handle at the specified offset.
    pub async fn write_handle(
        &self,
        handle_id: i64,
        data: &[u8],
        offset: i64,
    ) -> Result<i32, AgfsError> {
        let url = format!(
            "{}/handles/{}/write?offset={}",
            self.base_url, handle_id, offset
        );

        let response = self
            .http_client
            .put(&url)
            .body(data.to_vec())
            .header("Content-Type", "application/octet-stream")
            .send()
            .await?;

        #[derive(serde::Deserialize)]
        struct WriteResult {
            #[serde(rename = "bytesWritten")]
            bytes_written: i32,
        }

        // Try to parse bytes written, otherwise assume all bytes were written
        let bytes = response.bytes().await?;
        if let Ok(result) = serde_json::from_slice::<WriteResult>(&bytes) {
            Ok(result.bytes_written)
        } else {
            Ok(data.len() as i32)
        }
    }

    /// Seek within a file handle
    ///
    /// Seeks to a position in the file handle.
    ///
    /// # Arguments
    /// - `handle_id`: The handle ID
    /// - `offset`: The offset to seek to
    /// - `whence`: 0=from start, 1=from current, 2=from end
    pub async fn seek_handle(&self, handle_id: i64, offset: i64, whence: i32) -> Result<i64, AgfsError> {
        let url = format!(
            "{}/handles/{}/seek?offset={}&whence={}",
            self.base_url, handle_id, offset, whence
        );

        let response = self.http_client.post(&url).send().await?;

        #[derive(serde::Deserialize)]
        struct SeekResult {
            offset: i64,
        }

        let result: SeekResult = self.handle_response(response).await?;
        Ok(result.offset)
    }

    /// Sync a file handle
    ///
    /// Flushes pending writes to storage.
    pub async fn sync_handle(&self, handle_id: i64) -> Result<(), AgfsError> {
        let url = format!("{}/handles/{}/sync", self.base_url, handle_id);
        let response = self.http_client.post(&url).send().await?;
        self.handle_response(response).await
    }

    /// Get file info via handle
    ///
    /// Returns file information for the file associated with the handle.
    pub async fn stat_handle(&self, handle_id: i64) -> Result<FileInfo, AgfsError> {
        let url = format!("{}/handles/{}/stat", self.base_url, handle_id);
        let response = self.http_client.get(&url).send().await?;
        self.handle_response(response).await
    }

    /// Get handle information
    ///
    /// Returns metadata about the handle itself.
    pub async fn get_handle(&self, handle_id: i64) -> Result<HandleInfo, AgfsError> {
        let url = format!("{}/handles/{}", self.base_url, handle_id);
        let response = self.http_client.get(&url).send().await?;
        self.handle_response(response).await
    }

    // --- Plugin management ---

    /// List all plugins
    ///
    /// Returns a list of all mounted plugins.
    pub async fn list_plugins(&self) -> Result<Vec<Value>, AgfsError> {
        let url = format!("{}/plugins", self.base_url);
        let response = self.http_client.get(&url).send().await?;

        #[derive(serde::Deserialize)]
        struct PluginsResponse {
            plugins: Vec<Value>,
        }

        let resp: PluginsResponse = self.handle_response(response).await?;
        Ok(resp.plugins)
    }

    /// Get plugin information
    ///
    /// Returns information about a specific plugin.
    pub async fn get_plugin(&self, name: &str) -> Result<Value, AgfsError> {
        let url = format!("{}/plugins/{}", self.base_url, name);
        let response = self.http_client.get(&url).send().await?;
        self.handle_response(response).await
    }

    /// Create a new plugin instance
    ///
    /// Creates and mounts a new plugin instance at the specified path.
    pub async fn create_plugin(
        &self,
        name: &str,
        path: &str,
        config: &HashMap<String, Value>,
    ) -> Result<(), AgfsError> {
        let url = format!("{}/plugins", self.base_url);

        let mut body_data = serde_json::Map::new();
        body_data.insert("name".to_string(), Value::String(name.to_string()));
        body_data.insert("path".to_string(), Value::String(path.to_string()));

        // Convert HashMap to serde_json::Map
        let config_map: serde_json::Map<_, _> = config
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        body_data.insert("config".to_string(), Value::Object(config_map));

        let body = serde_json::to_string(&body_data)?;

        let response = self
            .http_client
            .post(&url)
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Delete a plugin instance
    ///
    /// Deletes and unmounts the plugin at the specified path.
    pub async fn delete_plugin(&self, path: &str) -> Result<(), AgfsError> {
        let url = format!("{}/plugins?path={}", self.base_url, encode_path(path));
        let response = self.http_client.delete(&url).send().await?;
        self.handle_response(response).await
    }

    // --- Helper methods ---

    /// Handle an HTTP response, returning error if status indicates failure
    async fn handle_response<T: for<'de> serde::Deserialize<'de>>(
        &self,
        response: Response,
    ) -> Result<T, AgfsError> {
        let status = response.status();

        if status.is_success() {
            let bytes = response.bytes().await?;
            serde_json::from_slice(&bytes).map_err(AgfsError::from)
        } else if status.as_u16() == 501 {
            Err(AgfsError::NotSupported)
        } else {
            let bytes = response.bytes().await?;
            if let Ok(err_resp) = serde_json::from_slice::<ErrorResponse>(&bytes) {
                Err(AgfsError::Internal(format!(
                    "HTTP {}: {}",
                    status.as_u16(),
                    err_resp.error
                )))
            } else {
                Err(AgfsError::Internal(format!(
                    "HTTP {}: {}",
                    status.as_u16(),
                    String::from_utf8_lossy(&bytes)
                )))
            }
        }
    }

    /// Handle an HTTP response that returns raw bytes
    async fn handle_response_bytes(&self, response: Response) -> Result<Vec<u8>, AgfsError> {
        let status = response.status();

        if status.is_success() {
            Ok(response.bytes().await?.to_vec())
        } else if status.as_u16() == 501 {
            Err(AgfsError::NotSupported)
        } else {
            let bytes = response.bytes().await?;
            if let Ok(err_resp) = serde_json::from_slice::<ErrorResponse>(&bytes) {
                Err(AgfsError::Internal(format!(
                    "HTTP {}: {}",
                    status.as_u16(),
                    err_resp.error
                )))
            } else {
                Err(AgfsError::Internal(format!(
                    "HTTP {}: {}",
                    status.as_u16(),
                    String::from_utf8_lossy(&bytes)
                )))
            }
        }
    }
}

/// Normalize the base URL to ensure it ends with /api/v1
fn normalize_base_url(mut base_url: String) -> String {
    // Remove trailing slash
    if base_url.ends_with('/') {
        base_url.pop();
    }

    // Check for :// to ensure we have a protocol
    if !base_url.contains("://") {
        // Invalid URL, return as-is and let HTTP client fail
        return base_url;
    }

    // Auto-append /api/v1 if not present
    if !base_url.ends_with("/api/v1") {
        base_url.push_str("/api/v1");
    }

    base_url
}

/// URL-encode a path for use in query parameters
fn encode_path(path: &str) -> String {
    urlencoding::encode(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_base_url() {
        assert_eq!(
            normalize_base_url("http://localhost:8080".to_string()),
            "http://localhost:8080/api/v1"
        );
        assert_eq!(
            normalize_base_url("http://localhost:8080/".to_string()),
            "http://localhost:8080/api/v1"
        );
        assert_eq!(
            normalize_base_url("http://localhost:8080/api/v1".to_string()),
            "http://localhost:8080/api/v1"
        );
        assert_eq!(
            normalize_base_url("http://localhost:8080/api/v1/".to_string()),
            "http://localhost:8080/api/v1"
        );
    }

    #[test]
    fn test_write_flag_bits() {
        let flags = WriteFlag::APPEND | WriteFlag::CREATE;
        assert!(flags.contains(WriteFlag::APPEND));
        assert!(flags.contains(WriteFlag::CREATE));
        assert!(!flags.contains(WriteFlag::TRUNCATE));
    }

    #[test]
    fn test_open_flag_access_mode() {
        assert_eq!(OpenFlag::RDONLY.access_mode(), 0);
        assert_eq!(OpenFlag::WRONLY.access_mode(), 1);
        assert_eq!(OpenFlag::RDWR.access_mode(), 2);
    }

    #[test]
    fn test_file_info_builder() {
        let file = FileInfo::file("test.txt", 100, 0o644);
        assert_eq!(file.name, "test.txt");
        assert_eq!(file.size, 100);
        assert!(!file.is_dir);
        assert!(!file.is_symlink);

        let dir = FileInfo::dir("testdir", 0o755);
        assert_eq!(dir.name, "testdir");
        assert!(dir.is_dir);

        let link = FileInfo::symlink("link.txt", "target.txt");
        assert_eq!(link.name, "link.txt");
        assert!(link.is_symlink);
        assert_eq!(link.meta.r#type, "symlink");
    }
}
