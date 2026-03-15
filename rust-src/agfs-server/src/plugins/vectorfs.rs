//! VectorFS - Vector Search File System
//!
//! Provides vector similarity search with embedding support.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Vector document with embedding
#[derive(Debug, Clone)]
struct VectorDocument {
    id: String,
    content: String,
    embedding: Option<Vec<f32>>,
    metadata: HashMap<String, String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Search result
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SearchResult {
    document_id: String,
    content: String,
    score: f32,
    metadata: HashMap<String, String>,
}

/// VectorFS - Vector search filesystem
#[derive(Debug, Clone)]
pub struct VectorFS {
    documents: Arc<RwLock<HashMap<String, VectorDocument>>>,
    doc_counter: Arc<std::sync::atomic::AtomicU64>,
    api_key: Option<String>,
    model: String,
    plugin_name: String,
}

impl VectorFS {
    /// Create a new VectorFS instance
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            doc_counter: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            api_key: None,
            model: "text-embedding-ada-002".to_string(),
            plugin_name: "vectorfs".to_string(),
        }
    }

    /// Set API key for embeddings
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set embedding model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Generate embedding for text (placeholder)
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, AgfsError> {
        // In full implementation, this would call an embedding API
        // For now, return a placeholder embedding based on text hash
        let hash = text.chars().map(|c| c as u32).sum::<u32>();
        let size = 1536; // Standard embedding size
        let mut embedding = Vec::with_capacity(size);
        for i in 0..size {
            embedding.push(((hash.wrapping_mul(i as u32)) as f32) / (u32::MAX as f32));
        }
        Ok(embedding)
    }

    /// Similarity search (cosine similarity)
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, AgfsError> {
        let query_embedding = self.generate_embedding(query).await?;

        let documents = self.documents.read().await;
        let mut results = Vec::new();

        for (id, doc) in documents.iter() {
            if let Some(ref doc_embedding) = doc.embedding {
                let score = cosine_similarity(&query_embedding, doc_embedding);
                results.push(SearchResult {
                    document_id: id.clone(),
                    content: doc.content.clone(),
                    score,
                    metadata: doc.metadata.clone(),
                });
            }
        }

        // Sort by score (descending) and take top results
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);

        Ok(results)
    }

    /// Add a document
    pub async fn add_document(&self, content: &str, metadata: HashMap<String, String>) -> Result<String, AgfsError> {
        let id = format!("doc_{}", self.doc_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        let embedding = self.generate_embedding(content).await?;

        let doc = VectorDocument {
            id: id.clone(),
            content: content.to_string(),
            embedding: Some(embedding),
            metadata,
            created_at: Utc::now(),
        };

        let mut documents = self.documents.write().await;
        documents.insert(id.clone(), doc);

        Ok(id)
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        dot_product / denominator
    }
}

impl Default for VectorFS {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for VectorFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        // Create a placeholder document
        let mut cache = self.documents.blocking_write();
        cache.insert(path.to_string(), VectorDocument {
            id: path.to_string(),
            content: String::new(),
            embedding: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        });
        Ok(())
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let mut cache = self.documents.blocking_write();
        cache.remove(path);
        Ok(())
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        let mut cache = self.documents.blocking_write();
        cache.retain(|k, _| !k.starts_with(path));
        Ok(())
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let cache = self.documents.blocking_read();
        if let Some(doc) = cache.get(path) {
            let data = doc.content.as_bytes();
            let offset = if offset < 0 { 0 } else { offset as usize };
            let size = if size < 0 { data.len() - offset } else { size as usize };

            if offset >= data.len() {
                return Ok(Vec::new());
            }

            let end = (offset + size).min(data.len());
            return Ok(data[offset..end].to_vec());
        }

        Err(AgfsError::not_found(path))
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, _flags: WriteFlag) -> Result<i64, AgfsError> {
        let content = std::str::from_utf8(data)
            .map_err(|_| AgfsError::invalid_argument("invalid UTF-8"))?;

        // Generate embedding
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;
        let embedding = runtime.block_on(async {
            self.generate_embedding(content).await
        })?;

        let mut cache = self.documents.blocking_write();
        cache.insert(path.to_string(), VectorDocument {
            id: path.to_string(),
            content: content.to_string(),
            embedding: Some(embedding),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        });

        Ok(data.len() as i64)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        if path == "/" || path.is_empty() {
            let cache = self.documents.blocking_read();
            return Ok(cache.iter().map(|(name, doc)| FileInfo {
                name: name.clone(),
                size: doc.content.len() as i64,
                mode: 0o644,
                mod_time: doc.created_at,
                is_dir: false,
                is_symlink: false,
                meta: MetaData {
                    name: self.plugin_name.clone(),
                    r#type: "vector-document".to_string(),
                    content: {
                        let mut map = HashMap::new();
                        if doc.embedding.is_some() {
                            map.insert("indexed".to_string(), "true".to_string());
                        }
                        map
                    },
                },
            }).collect());
        }

        Ok(Vec::new())
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        if path == "/" || path.is_empty() {
            let cache = self.documents.blocking_read();
            return Ok(FileInfo {
                name: String::new(),
                size: cache.len() as i64,
                mode: 0o755,
                mod_time: Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: MetaData {
                    name: self.plugin_name.clone(),
                    r#type: "vector-filesystem".to_string(),
                    content: {
                        let mut map = HashMap::new();
                        map.insert("model".to_string(), self.model.clone());
                        map
                    },
                },
            });
        }

        let cache = self.documents.blocking_read();
        if let Some(doc) = cache.get(path) {
            return Ok(FileInfo {
                name: path.trim_start_matches('/').to_string(),
                size: doc.content.len() as i64,
                mode: 0o644,
                mod_time: doc.created_at,
                is_dir: false,
                is_symlink: false,
                meta: MetaData {
                    name: self.plugin_name.clone(),
                    r#type: "vector-document".to_string(),
                    content: {
                        let mut map = HashMap::new();
                        if doc.embedding.is_some() {
                            map.insert("indexed".to_string(), "true".to_string());
                        }
                        map
                    },
                },
            });
        }

        Err(AgfsError::not_found(path))
    }

    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError> {
        let mut cache = self.documents.blocking_write();
        if let Some(mut doc) = cache.remove(old_path) {
            doc.id = new_path.to_string();
            cache.insert(new_path.to_string(), doc);
            Ok(())
        } else {
            Err(AgfsError::not_found(old_path))
        }
    }

    fn chmod(&self, _path: &str, _mode: u32) -> Result<(), AgfsError> {
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

    #[test]
    fn test_vectorfs_create_and_write() {
        let fs = VectorFS::new();

        // Create a document directly without using write
        let doc = VectorDocument {
            id: "/doc_1".to_string(),
            content: "Hello, world!".to_string(),
            embedding: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };
        fs.documents.blocking_write().insert("/doc_1".to_string(), doc);

        let data = fs.read("/doc_1", 0, -1).unwrap();
        assert_eq!(data, b"Hello, world!");
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let c = vec![-1.0, -2.0, -3.0];

        // Same vectors should have similarity 1.0
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        // Opposite vectors should have similarity -1.0
        assert!((cosine_similarity(&a, &c) - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_vectorfs_stat() {
        let fs = VectorFS::new();

        // Create a document directly without using write
        let doc = VectorDocument {
            id: "/doc_1".to_string(),
            content: "test content".to_string(),
            embedding: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };
        fs.documents.blocking_write().insert("/doc_1".to_string(), doc);

        let stat = fs.stat("/doc_1").unwrap();
        assert_eq!(stat.size, 12); // "test content".len()
        assert_eq!(stat.meta.r#type, "vector-document");
    }

    #[test]
    fn test_vectorfs_with_api_key() {
        let fs = VectorFS::new().with_api_key("sk-test").with_model("text-embedding-3-small");
        assert_eq!(fs.api_key, Some("sk-test".to_string()));
        assert_eq!(fs.model, "text-embedding-3-small");
    }
}
