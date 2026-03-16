//! VectorFS - Vector Search File System
//!
//! Provides vector similarity search with embedding support.
//! Supports OpenAI embeddings API and compatible endpoints.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Embedding API configuration
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// API key for authentication
    pub api_key: Option<String>,
    /// Model to use (e.g., "text-embedding-3-small", "text-embedding-ada-002")
    pub model: String,
    /// API base URL (default: https://api.openai.com/v1)
    pub api_base: String,
    /// Embedding dimensions (auto-detected from API response)
    pub dimensions: Option<usize>,
    /// Request timeout
    pub timeout: Duration,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY")
                .or_else(|_| std::env::var("VECTORFS_API_KEY"))
                .ok(),
            model: "text-embedding-3-small".to_string(),
            api_base: "https://api.openai.com/v1".to_string(),
            dimensions: None,
            timeout: Duration::from_secs(30),
        }
    }
}

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
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub document_id: String,
    pub content: String,
    pub score: f32,
    pub metadata: HashMap<String, String>,
}

/// OpenAI Embedding API request
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: String,
    model: String,
    dimensions: Option<usize>,
}

/// OpenAI Embedding API response
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    usage: Option<EmbeddingUsage>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Deserialize)]
struct EmbeddingUsage {
    total_tokens: u32,
}

/// Error response from API
#[derive(Debug, Deserialize)]
struct ApiError {
    error: ErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    message: String,
    #[serde(default)]
    r#type: String,
}

/// VectorFS - Vector search filesystem
#[derive(Debug, Clone)]
pub struct VectorFS {
    documents: Arc<RwLock<HashMap<String, VectorDocument>>>,
    doc_counter: Arc<std::sync::atomic::AtomicU64>,
    config: EmbeddingConfig,
    plugin_name: String,
    http_client: Arc<reqwest::Client>,
}

impl VectorFS {
    /// Create a new VectorFS instance with default configuration
    pub fn new() -> Self {
        Self::with_config(EmbeddingConfig::default())
    }

    /// Create a new VectorFS instance with custom configuration
    pub fn with_config(config: EmbeddingConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            doc_counter: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            config,
            plugin_name: "vectorfs".to_string(),
            http_client: Arc::new(http_client),
        }
    }

    /// Set API key for embeddings
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.config.api_key = Some(api_key.into());
        self
    }

    /// Set embedding model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    /// Set API base URL (for compatible APIs)
    pub fn with_api_base(mut self, api_base: impl Into<String>) -> Self {
        self.config.api_base = api_base.into();
        self
    }

    /// Set embedding dimensions
    pub fn with_dimensions(mut self, dimensions: usize) -> Self {
        self.config.dimensions = Some(dimensions);
        self
    }

    /// Generate embedding for text using the configured API
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, AgfsError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| AgfsError::internal("VectorFS: API key not configured. Set OPENAI_API_KEY environment variable or use with_api_key()"))?;

        let url = format!("{}/embeddings", self.config.api_base.trim_end_matches('/'));

        let request_body = EmbeddingRequest {
            input: text.to_string(),
            model: self.config.model.clone(),
            dimensions: self.config.dimensions,
        };

        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AgfsError::internal(format!("VectorFS: API request failed: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| AgfsError::internal(format!("VectorFS: failed to read response: {}", e)))?;

        if !status.is_success() {
            // Try to parse error message
            if let Ok(api_err) = serde_json::from_str::<ApiError>(&response_text) {
                return Err(AgfsError::internal(format!("VectorFS API error: {}", api_err.error.message)));
            }
            return Err(AgfsError::internal(format!("VectorFS: API returned error {}: {}", status.as_u16(), response_text)));
        }

        let embedding_response: EmbeddingResponse = serde_json::from_str(&response_text)
            .map_err(|e| AgfsError::internal(format!("VectorFS: failed to parse response: {}", e)))?;

        // Get the first embedding (we only sent one input)
        let embedding = embedding_response.data
            .first()
            .ok_or_else(|| AgfsError::internal("VectorFS: API returned no embeddings"))?
            .embedding
            .clone();

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

    /// Add a document with embedding
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

    /// Get document by ID
    pub async fn get_document(&self, id: &str) -> Option<VectorDocument> {
        let documents = self.documents.read().await;
        documents.get(id).cloned()
    }

    /// List all document IDs
    pub async fn list_documents(&self) -> Vec<String> {
        let documents = self.documents.read().await;
        documents.keys().cloned().collect()
    }

    /// Delete a document
    pub async fn delete_document(&self, id: &str) -> bool {
        let mut documents = self.documents.write().await;
        documents.remove(id).is_some()
    }

    /// Get the number of documents
    pub async fn document_count(&self) -> usize {
        let documents = self.documents.read().await;
        documents.len()
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
                        map.insert("model".to_string(), self.config.model.clone());
                        map.insert("api_base".to_string(), self.config.api_base.clone());
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
                            map.insert("embedding_dim".to_string(), doc.embedding.as_ref().unwrap().len().to_string());
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
    fn test_vectorfs_create_and_cache() {
        let fs = VectorFS::new();

        fs.create("/doc_1").unwrap();

        let cache = fs.documents.blocking_read();
        assert!(cache.contains_key("/doc_1"));
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
            embedding: Some(vec![0.1, 0.2, 0.3]),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };
        fs.documents.blocking_write().insert("/doc_1".to_string(), doc);

        let stat = fs.stat("/doc_1").unwrap();
        assert_eq!(stat.size, 12); // "test content".len()
        assert_eq!(stat.meta.r#type, "vector-document");
    }

    #[test]
    fn test_vectorfs_config() {
        let config = EmbeddingConfig {
            api_key: Some("sk-test".to_string()),
            model: "text-embedding-3-small".to_string(),
            api_base: "https://api.openai.com/v1".to_string(),
            dimensions: Some(1536),
            timeout: Duration::from_secs(60),
        };
        let fs = VectorFS::with_config(config);
        assert_eq!(fs.config.api_key, Some("sk-test".to_string()));
        assert_eq!(fs.config.model, "text-embedding-3-small");
        assert_eq!(fs.config.dimensions, Some(1536));
    }

    #[test]
    fn test_vectorfs_builder_pattern() {
        let fs = VectorFS::new()
            .with_api_key("sk-test")
            .with_model("text-embedding-3-large")
            .with_dimensions(3072)
            .with_api_base("https://api.example.com/v1");

        assert_eq!(fs.config.api_key, Some("sk-test".to_string()));
        assert_eq!(fs.config.model, "text-embedding-3-large");
        assert_eq!(fs.config.dimensions, Some(3072));
        assert_eq!(fs.config.api_base, "https://api.example.com/v1");
    }

    #[tokio::test]
    #[ignore = "requires API key"]
    async fn test_vectorfs_real_embedding() {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY must be set for this test");

        let config = EmbeddingConfig {
            api_key: Some(api_key),
            ..Default::default()
        };

        let fs = VectorFS::with_config(config);
        let embedding = fs.generate_embedding("hello world").await.unwrap();

        assert!(!embedding.is_empty());
        assert!(embedding.len() > 100); // Embeddings should have significant dimensionality
        println!("Embedding dimension: {}", embedding.len());
    }

    #[tokio::test]
    #[ignore = "requires API key"]
    async fn test_vectorfs_search() {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY must be set for this test");

        let config = EmbeddingConfig {
            api_key: Some(api_key),
            ..Default::default()
        };

        let fs = VectorFS::with_config(config);

        // Add some documents
        let mut metadata = HashMap::new();
        metadata.insert("category".to_string(), "rust".to_string());
        fs.add_document("Rust is a systems programming language", metadata).await.unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("category".to_string(), "python".to_string());
        fs.add_document("Python is a high-level programming language", metadata).await.unwrap();

        // Search for similar content
        let results = fs.search("systems programming", 5).await.unwrap();

        assert!(!results.is_empty());
        println!("Search results:");
        for result in &results {
            println!("  - {} (score: {:.4}): {}", result.document_id, result.score, result.content);
        }
    }
}
