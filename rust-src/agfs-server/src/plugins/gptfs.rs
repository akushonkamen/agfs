//! GptFS - GPT Integration File System
//!
//! Write prompts to files, get GPT responses as file content.
//! Supports OpenAI API and compatible endpoints.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// OpenAI API configuration
#[derive(Debug, Clone)]
pub struct GptConfig {
    /// API key for authentication
    pub api_key: Option<String>,
    /// Model to use (e.g., "gpt-4", "gpt-3.5-turbo", "claude-3-opus")
    pub model: String,
    /// API base URL (default: https://api.openai.com/v1)
    pub api_base: String,
    /// Maximum tokens in response
    pub max_tokens: Option<u32>,
    /// Temperature for response generation
    pub temperature: Option<f32>,
    /// Request timeout
    pub timeout: Duration,
}

impl Default for GptConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY")
                .or_else(|_| std::env::var("GPTFS_API_KEY"))
                .ok(),
            model: "gpt-3.5-turbo".to_string(),
            api_base: "https://api.openai.com/v1".to_string(),
            max_tokens: Some(2048),
            temperature: Some(0.7),
            timeout: Duration::from_secs(60),
        }
    }
}

/// GPT response cache entry
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GptCacheEntry {
    prompt: String,
    response: String,
    created_at: chrono::DateTime<chrono::Utc>,
    tokens_used: u32,
    model: String,
}

/// OpenAI API request body
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

/// Chat message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI API response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

/// Individual choice in the response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Choice {
    message: ChatMessage,
    finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Usage {
    total_tokens: u32,
    prompt_tokens: u32,
    completion_tokens: u32,
}

/// Error response from API
#[derive(Debug, Deserialize)]
struct ApiError {
    error: ErrorDetail,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ErrorDetail {
    message: String,
    #[serde(default)]
    r#type: String,
}

/// GptFS - GPT integration filesystem
#[derive(Debug, Clone)]
pub struct GptFS {
    cache: Arc<RwLock<HashMap<String, GptCacheEntry>>>,
    config: GptConfig,
    plugin_name: String,
    http_client: Arc<reqwest::Client>,
}

impl GptFS {
    /// Create a new GptFS instance with default configuration
    pub fn new() -> Self {
        Self::with_config(GptConfig::default())
    }

    /// Create a new GptFS instance with custom configuration
    pub fn with_config(config: GptConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            plugin_name: "gptfs".to_string(),
            http_client: Arc::new(http_client),
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.config.api_key = Some(api_key.into());
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    /// Set API base URL (for compatible APIs like Anthropic, local models, etc.)
    pub fn with_api_base(mut self, api_base: impl Into<String>) -> Self {
        self.config.api_base = api_base.into();
        self
    }

    /// Set maximum tokens for response
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.config.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature for response generation
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.config.temperature = Some(temperature);
        self
    }

    /// Generate a response for a prompt using the configured API
    pub async fn generate(&self, prompt: &str) -> Result<String, AgfsError> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| AgfsError::internal("GptFS: API key not configured. Set GPTFS_API_KEY environment variable or use with_api_key()"))?;

        let url = format!("{}/chat/completions", self.config.api_base.trim_end_matches('/'));

        let request_body = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                }
            ],
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
        };

        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AgfsError::internal(format!("GptFS: API request failed: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| AgfsError::internal(format!("GptFS: failed to read response: {}", e)))?;

        if !status.is_success() {
            // Try to parse error message
            if let Ok(api_err) = serde_json::from_str::<ApiError>(&response_text) {
                return Err(AgfsError::internal(format!("GptFS API error: {}", api_err.error.message)));
            }
            return Err(AgfsError::internal(format!("GptFS: API returned error {}: {}", status.as_u16(), response_text)));
        }

        let chat_response: ChatResponse = serde_json::from_str(&response_text)
            .map_err(|e| AgfsError::internal(format!("GptFS: failed to parse response: {}", e)))?;

        let content = chat_response.choices
            .first()
            .and_then(|c| Some(c.message.content.clone()))
            .unwrap_or_else(|| String::from("(No response content)"));

        Ok(content)
    }

    /// Count tokens in text (simplified estimation)
    /// For accurate counting, use tiktoken-rs or call the API
    pub fn count_tokens(text: &str) -> usize {
        // Rough estimate: ~4 characters per token for English text
        // This is a rough approximation - actual tokenization varies by model
        text.chars().count().div_ceil(4)
    }
}

impl Default for GptFS {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for GptFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        // Create a new prompt file (empty initially)
        let mut cache = self.cache.blocking_write();
        cache.insert(path.to_string(), GptCacheEntry {
            prompt: String::new(),
            response: String::new(),
            created_at: Utc::now(),
            tokens_used: 0,
            model: self.config.model.clone(),
        });
        Ok(())
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let mut cache = self.cache.blocking_write();
        cache.remove(path);
        Ok(())
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        let mut cache = self.cache.blocking_write();
        cache.retain(|k, _| !k.starts_with(path));
        Ok(())
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let cache = self.cache.blocking_read();
        if let Some(entry) = cache.get(path) {
            let data = entry.response.as_bytes();
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
        let prompt = std::str::from_utf8(data)
            .map_err(|_| AgfsError::invalid_argument("invalid UTF-8"))?;

        // Generate response using the GPT API
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        let response = runtime.block_on(async move {
            self.generate(prompt).await
        })?;

        let tokens_used = Self::count_tokens(prompt) as u32;

        let mut cache = self.cache.blocking_write();
        cache.insert(path.to_string(), GptCacheEntry {
            prompt: prompt.to_string(),
            response,
            created_at: Utc::now(),
            tokens_used,
            model: self.config.model.clone(),
        });

        Ok(data.len() as i64)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        if path == "/" || path.is_empty() {
            let cache = self.cache.blocking_read();
            return Ok(cache.iter().map(|(name, entry)| FileInfo {
                name: name.clone(),
                size: entry.response.len() as i64,
                mode: 0o644,
                mod_time: entry.created_at,
                is_dir: false,
                is_symlink: false,
                meta: MetaData {
                    name: self.plugin_name.clone(),
                    r#type: "gpt-response".to_string(),
                    content: {
                        let mut map = HashMap::new();
                        map.insert("model".to_string(), entry.model.clone());
                        map.insert("tokens".to_string(), entry.tokens_used.to_string());
                        map
                    },
                },
            }).collect());
        }

        Ok(Vec::new())
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        if path == "/" || path.is_empty() {
            let cache = self.cache.blocking_read();
            return Ok(FileInfo {
                name: String::new(),
                size: cache.len() as i64,
                mode: 0o755,
                mod_time: Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: MetaData {
                    name: self.plugin_name.clone(),
                    r#type: "gpt-filesystem".to_string(),
                    content: {
                        let mut map = HashMap::new();
                        map.insert("model".to_string(), self.config.model.clone());
                        map.insert("api_base".to_string(), self.config.api_base.clone());
                        map
                    },
                },
            });
        }

        let cache = self.cache.blocking_read();
        if let Some(entry) = cache.get(path) {
            return Ok(FileInfo {
                name: path.trim_start_matches('/').to_string(),
                size: entry.response.len() as i64,
                mode: 0o644,
                mod_time: entry.created_at,
                is_dir: false,
                is_symlink: false,
                meta: MetaData {
                    name: self.plugin_name.clone(),
                    r#type: "gpt-response".to_string(),
                    content: {
                        let mut map = HashMap::new();
                        map.insert("model".to_string(), entry.model.clone());
                        map.insert("tokens".to_string(), entry.tokens_used.to_string());
                        map
                    },
                },
            });
        }

        Err(AgfsError::not_found(path))
    }

    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError> {
        let mut cache = self.cache.blocking_write();
        if let Some(entry) = cache.remove(old_path) {
            cache.insert(new_path.to_string(), entry);
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
    fn test_gptfs_create_and_cache() {
        let fs = GptFS::new();

        fs.create("/prompt1").unwrap();

        let cache = fs.cache.blocking_read();
        assert!(cache.contains_key("/prompt1"));
    }

    #[test]
    fn test_gptfs_count_tokens() {
        assert_eq!(GptFS::count_tokens("hello world"), 3);
        assert_eq!(GptFS::count_tokens(""), 0);
        assert_eq!(GptFS::count_tokens("a"), 1);
    }

    #[test]
    fn test_gptfs_read_dir() {
        let fs = GptFS::new();

        fs.create("/prompt1").unwrap();

        let files = fs.read_dir("/").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "/prompt1");
    }

    #[test]
    fn test_gptfs_config() {
        let config = GptConfig {
            api_key: Some("sk-test".to_string()),
            model: "gpt-4".to_string(),
            api_base: "https://api.openai.com/v1".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            timeout: Duration::from_secs(30),
        };
        let fs = GptFS::with_config(config);
        assert_eq!(fs.config.api_key, Some("sk-test".to_string()));
        assert_eq!(fs.config.model, "gpt-4");
        assert_eq!(fs.config.max_tokens, Some(4096));
    }

    #[test]
    fn test_gptfs_builder_pattern() {
        let fs = GptFS::new()
            .with_api_key("sk-test")
            .with_model("gpt-4")
            .with_max_tokens(1000)
            .with_temperature(0.8)
            .with_api_base("https://api.example.com/v1");

        assert_eq!(fs.config.api_key, Some("sk-test".to_string()));
        assert_eq!(fs.config.model, "gpt-4");
        assert_eq!(fs.config.max_tokens, Some(1000));
        assert_eq!(fs.config.temperature, Some(0.8));
        assert_eq!(fs.config.api_base, "https://api.example.com/v1");
    }

    #[tokio::test]
    #[ignore = "requires API key"]
    async fn test_gptfs_real_api_call() {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY must be set for this test");

        let config = GptConfig {
            api_key: Some(api_key),
            model: "gpt-3.5-turbo".to_string(),
            ..Default::default()
        };

        let fs = GptFS::with_config(config);
        let response = fs.generate("Say 'test passed'").await.unwrap();

        assert!(!response.is_empty());
        println!("Response: {}", response);
    }
}
