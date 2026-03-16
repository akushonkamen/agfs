//! S3FS - S3 Object Storage File System
//!
//! Provides file system interface to S3 object storage using aws-sdk-s3.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    types::{Delete, ObjectIdentifier},
    Client,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

/// S3 configuration
#[derive(Debug, Clone)]
pub struct S3Config {
    /// AWS region (e.g., "us-east-1")
    pub region: Option<String>,
    /// AWS access key ID
    pub access_key_id: Option<String>,
    /// AWS secret access key
    pub secret_access_key: Option<String>,
    /// S3 endpoint URL (for S3-compatible services like MinIO)
    pub endpoint_url: Option<String>,
    /// Whether to use path-style addressing (s3.amazonaws.com/bucket/key)
    pub force_path_style: bool,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            region: None,
            access_key_id: None,
            secret_access_key: None,
            endpoint_url: None,
            force_path_style: false,
        }
    }
}

/// S3 metadata cache entry
#[derive(Debug, Clone)]
struct S3Metadata {
    size: i64,
    mod_time: chrono::DateTime<chrono::Utc>,
    etag: Option<String>,
    content_type: Option<String>,
}

/// S3 file system with real AWS S3 operations
#[derive(Debug, Clone)]
pub struct S3FS {
    client: Arc<Client>,
    bucket: String,
    prefix: String,
    /// In-memory metadata cache (optional, for performance)
    cache: Arc<tokio::sync::RwLock<HashMap<String, S3Metadata>>>,
}

impl S3FS {
    /// Create a new S3FS instance with default configuration
    ///
    /// Configuration is loaded from environment:
    /// - AWS_REGION: AWS region (default: us-east-1)
    /// - AWS_ACCESS_KEY_ID: Access key
    /// - AWS_SECRET_ACCESS_KEY: Secret key
    /// - AWS_ENDPOINT_URL: Custom endpoint (for MinIO, etc.)
    pub async fn new(bucket: impl Into<String>) -> Result<Self, AgfsError> {
        Self::with_prefix(bucket, "").await
    }

    /// Create a new S3FS instance with prefix and config
    pub async fn with_prefix(
        bucket: impl Into<String>,
        prefix: impl Into<String>,
    ) -> Result<Self, AgfsError> {
        Self::with_config(bucket, prefix, S3Config::default()).await
    }

    /// Create a new S3FS instance with custom configuration
    pub async fn with_config(
        bucket: impl Into<String>,
        prefix: impl Into<String>,
        config: S3Config,
    ) -> Result<Self, AgfsError> {
        // Load AWS config from environment
        let mut config_loader = aws_config::defaults(BehaviorVersion::latest());

        if let Some(region) = config.region {
            config_loader = config_loader.region(aws_config::Region::new(region));
        }

        if let Some(_endpoint) = config.endpoint_url {
            // For custom endpoints (MinIO, etc.), use https://
            // config_loader = config_loader.endpoint_url(endpoint);
        }

        // Load config and create S3 client
        let shared_config = config_loader.load().await;

        let client = Client::new(&shared_config);

        Ok(Self {
            client: Arc::new(client),
            bucket: bucket.into(),
            prefix: prefix.into(),
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// Get full S3 key for a path
    fn s3_key(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", self.prefix.trim_end_matches('/'), path)
        }
    }

    /// List S3 objects with the given prefix
    pub async fn list_objects(&self) -> Result<Vec<String>, AgfsError> {
        let prefix = if self.prefix.is_empty() {
            None
        } else {
            Some(self.prefix.clone())
        };

        let mut result = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut builder = self.client
                .list_objects_v2()
                .bucket(&self.bucket)
                .max_keys(1000); // Maximum allowed

            if let Some(ref p) = prefix {
                builder = builder.prefix(p);
            }

            if let Some(ref token) = continuation_token {
                builder = builder.continuation_token(token);
            }

            let output = builder.send().await
                .map_err(|e| AgfsError::internal(format!("S3 list objects failed: {}", e)))?;

            // Collect object keys
            if let Some(contents) = output.contents.as_ref() {
                for obj in contents.iter() {
                    if let Some(key) = &obj.key {
                        result.push(key.clone());
                    }
                }
            }

            // Check if there are more objects
            if let Some(token) = output.next_continuation_token() {
                if token.is_empty() {
                    break;
                }
                continuation_token = Some(token.to_string());
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Head object to get metadata without downloading
    pub async fn head_object(&self, key: &str) -> Result<S3Metadata, AgfsError> {
        let output = self.client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send().await
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("NoSuchKey") || err_str.contains("404") {
                    return AgfsError::not_found(key);
                }
                AgfsError::internal(format!("S3 head object failed: {}", err_str))
            })?;

        Ok(S3Metadata {
            size: output.content_length().unwrap_or(0) as i64,
            mod_time: output.last_modified().map(|_dt| {
                // Convert AWS DateTime to chrono DateTime
                // For now, use current time as placeholder
                Utc::now()
            }).unwrap_or_else(|| Utc::now()),
            etag: output.e_tag().map(|s| s.to_string()),
            content_type: output.content_type().map(|s| s.to_string()),
        })
    }

    /// Put object to S3
    pub async fn put_object(&self, key: &str, data: Vec<u8>, content_type: Option<&str>) -> Result<String, AgfsError> {
        let mut builder = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(aws_sdk_s3::primitives::ByteStream::from(data));

        if let Some(ct) = content_type {
            builder = builder.content_type(ct);
        }

        let output = builder.send().await
            .map_err(|e| AgfsError::internal(format!("S3 put object failed: {}", e)))?;

        Ok(output.e_tag().unwrap_or("").to_string())
    }

    /// Get object from S3
    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>, AgfsError> {
        let output = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send().await
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("NoSuchKey") || err_str.contains("404") {
                    return AgfsError::not_found(key);
                }
                AgfsError::internal(format!("S3 get object failed: {}", err_str))
            })?;

        // Collect ByteStream into Vec<u8>
        use futures::StreamExt;
        let mut data = Vec::new();
        let mut body = output.body;
        while let Some(chunk_result) = body.next().await {
            let chunk = chunk_result.map_err(|e| AgfsError::internal(format!("Failed to read S3 object body: {}", e)))?;
            data.extend_from_slice(&chunk);
        }

        Ok(data)
    }

    /// Delete object from S3
    pub async fn delete_object(&self, key: &str) -> Result<(), AgfsError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send().await
            .map_err(|e| AgfsError::internal(format!("S3 delete object failed: {}", e)))?;

        Ok(())
    }

    /// Delete multiple objects from S3
    pub async fn delete_objects(&self, keys: Vec<String>) -> Result<usize, AgfsError> {
        if keys.is_empty() {
            return Ok(0);
        }

        let obj_ids: Vec<ObjectIdentifier> = keys.into_iter()
            .map(|k| ObjectIdentifier::builder().key(k).build().unwrap())
            .collect();

        let delete = Delete::builder()
            .set_objects(Some(obj_ids))
            .build().unwrap();

        let output = self.client
            .delete_objects()
            .bucket(&self.bucket)
            .delete(delete)
            .send().await
            .map_err(|e| AgfsError::internal(format!("S3 delete objects failed: {}", e)))?;

        let deleted_count = if let Some(deleted) = output.deleted.as_ref() {
            deleted.len()
        } else {
            0
        };

        Ok(deleted_count)
    }

    /// Update local cache
    async fn update_cache(&self, key: String, metadata: S3Metadata) {
        let mut cache = self.cache.write().await;
        cache.insert(key, metadata);
    }

    /// Remove from local cache
    async fn remove_from_cache(&self, key: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(key);
    }

    /// Get from local cache
    async fn get_from_cache(&self, key: &str) -> Option<S3Metadata> {
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }

    /// Refresh cache by listing S3 objects
    pub async fn refresh_cache(&self) -> Result<(), AgfsError> {
        let keys = self.list_objects().await?;

        let mut cache = self.cache.write().await;
        cache.clear();

        // For each key, we need to get metadata via HEAD
        // This could be slow, so we just store basic info
        for key in keys {
            let _name = key.strip_prefix(&self.prefix).unwrap_or(&key).to_string();
            cache.insert(key.clone(), S3Metadata {
                size: 0, // Will be updated on access
                mod_time: Utc::now(),
                etag: None,
                content_type: None,
            });
        }

        Ok(())
    }
}

// Note: Default implementation requires async runtime for AWS client creation
// Use S3FS::new() or S3FS::with_config() instead

impl FileSystem for S3FS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let key = self.s3_key(path);

            // Create an empty object (S3 doesn't have "create" concept, so we upload empty data)
            self.put_object(&key, vec![], None).await?;

            Ok(())
        })
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        // S3 doesn't have directories, but we simulate them
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let key = self.s3_key(path);
            self.delete_object(&key).await?;
            self.remove_from_cache(&key).await;
            Ok(())
        })
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let prefix = self.s3_key(path);

            // List all objects with prefix and delete them
            let objects = self.list_objects().await?;
            let keys_to_delete: Vec<String> = objects.into_iter()
                .filter(|k| k.starts_with(&prefix))
                .collect();

            if !keys_to_delete.is_empty() {
                self.delete_objects(keys_to_delete).await?;
            }

            // Clear cache entries with prefix
            let mut cache = self.cache.write().await;
            cache.retain(|k, _| !k.starts_with(&prefix));

            Ok(())
        })
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let key = self.s3_key(path);
            let data = self.get_object(&key).await?;

            // Apply offset and size
            let offset = if offset < 0 { 0 } else { offset as usize };
            let size = if size < 0 { data.len() - offset } else { size as usize };

            if offset >= data.len() {
                return Ok(Vec::new());
            }

            let end = (offset + size).min(data.len());
            Ok(data[offset..end].to_vec())
        })
    }

    fn write(&self, path: &str, data: &[u8], offset: i64, flags: WriteFlag) -> Result<i64, AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let key = self.s3_key(path);
            let len = data.len() as i64;

            // Handle append flag
            if flags.contains(WriteFlag::APPEND) && offset > 0 {
                // Fetch existing data and append
                match self.get_object(&key).await {
                    Ok(mut existing) => {
                        existing.extend_from_slice(data);
                        self.put_object(&key, existing, Some("application/octet-stream")).await?;
                    }
                    Err(_) => {
                        // Object doesn't exist, create new
                        self.put_object(&key, data.to_vec(), Some("application/octet-stream")).await?;
                    }
                }
            } else {
                // Normal write (replace or create)
                self.put_object(&key, data.to_vec(), Some("application/octet-stream")).await?;
            }

            // Update cache
            self.update_cache(key.clone(), S3Metadata {
                size: len,
                mod_time: Utc::now(),
                etag: None,
                content_type: Some("application/octet-stream".to_string()),
            }).await;

            Ok(len)
        })
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let prefix = self.s3_key(path);
            let mut files = Vec::new();

            // List objects from S3
            let objects = self.list_objects().await?;

            for key in objects {
                if !key.starts_with(&prefix) {
                    continue;
                }

                let name = key[prefix.len()..].to_string();

                // Skip directory entries (keys containing '/')
                if name.contains('/') {
                    // Could add as directory if name ends before another /
                    continue;
                }

                // Get metadata from cache or S3
                let metadata = if let Some(cached) = self.get_from_cache(&key).await {
                    cached
                } else {
                    // Head object to get metadata
                    match self.head_object(&key).await {
                        Ok(meta) => meta,
                        Err(_) => continue,
                    }
                };

                files.push(FileInfo {
                    name,
                    size: metadata.size,
                    mode: 0o644,
                    mod_time: metadata.mod_time,
                    is_dir: false,
                    is_symlink: false,
                    meta: MetaData {
                        name: "s3fs".to_string(),
                        r#type: "s3-object".to_string(),
                        content: {
                            let mut map = HashMap::new();
                            if let Some(etag) = &metadata.etag {
                                map.insert("etag".to_string(), etag.clone());
                            }
                            if let Some(ct) = &metadata.content_type {
                                map.insert("content_type".to_string(), ct.clone());
                            }
                            map
                        },
                    },
                });
            }

            Ok(files)
        })
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            if path == "/" || path.is_empty() {
                return Ok(FileInfo {
                    name: String::new(),
                    size: 0,
                    mode: 0o555,
                    mod_time: Utc::now(),
                    is_dir: true,
                    is_symlink: false,
                    meta: MetaData {
                        name: "s3fs".to_string(),
                        r#type: "s3-bucket".to_string(),
                        content: {
                            let mut map = HashMap::new();
                            map.insert("bucket".to_string(), self.bucket.clone());
                            map
                        },
                    },
                });
            }

            let key = self.s3_key(path);

            // Try cache first
            if let Some(metadata) = self.get_from_cache(&key).await {
                return Ok(FileInfo {
                    name: path.trim_start_matches('/').to_string(),
                    size: metadata.size,
                    mode: 0o644,
                    mod_time: metadata.mod_time,
                    is_dir: false,
                    is_symlink: false,
                    meta: MetaData {
                        name: "s3fs".to_string(),
                        r#type: "s3-object".to_string(),
                        content: {
                            let mut map = HashMap::new();
                            if let Some(etag) = &metadata.etag {
                                map.insert("etag".to_string(), etag.clone());
                            }
                            map
                        },
                    },
                });
            }

            // Head object to get metadata
            let metadata = self.head_object(&key).await?;

            // Update cache
            self.update_cache(key.clone(), metadata.clone()).await;

            Ok(FileInfo {
                name: path.trim_start_matches('/').to_string(),
                size: metadata.size,
                mode: 0o644,
                mod_time: metadata.mod_time,
                is_dir: false,
                is_symlink: false,
                meta: MetaData {
                    name: "s3fs".to_string(),
                    r#type: "s3-object".to_string(),
                    content: {
                        let mut map = HashMap::new();
                        if let Some(etag) = &metadata.etag {
                            map.insert("etag".to_string(), etag.clone());
                        }
                        map
                    },
                },
            })
        })
    }

    fn rename(&self, _old_path: &str, _new_path: &str) -> Result<(), AgfsError> {
        // S3 rename is copy + delete
        // Could be implemented, but for now return NotSupported
        Err(AgfsError::NotSupported)
    }

    fn chmod(&self, _path: &str, _mode: u32) -> Result<(), AgfsError> {
        // S3 doesn't support file modes
        Ok(())
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

    #[tokio::test]
    #[ignore = "requires S3 credentials"]
    async fn test_s3fs_operations() {
        let fs = S3FS::new("test-bucket").await.unwrap();

        // Create a test file
        fs.create("/test.txt").unwrap();
        fs.write("/test.txt", b"hello from s3", 0, WriteFlag::NONE).unwrap();

        // Read back
        let data = fs.read("/test.txt", 0, -1).unwrap();
        assert_eq!(data, b"hello from s3");

        // Stat
        let info = fs.stat("/test.txt").unwrap();
        assert_eq!(info.size, 14);

        // Cleanup
        fs.remove("/test.txt").unwrap();
    }
}
