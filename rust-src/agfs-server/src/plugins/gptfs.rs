//! GptFS - GPT Integration File System
//!
//! Write prompts to files, get GPT responses as file content.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// GPT response cache entry
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GptCacheEntry {
    prompt: String,
    response: String,
    created_at: chrono::DateTime<chrono::Utc>,
    tokens_used: u32,
}

/// GptFS - GPT integration filesystem
#[derive(Debug, Clone)]
pub struct GptFS {
    cache: Arc<RwLock<HashMap<String, GptCacheEntry>>>,
    api_key: Option<String>,
    model: String,
    plugin_name: String,
}

impl GptFS {
    /// Create a new GptFS instance
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            api_key: None,
            model: "gpt-3.5-turbo".to_string(),
            plugin_name: "gptfs".to_string(),
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Generate a response for a prompt
    pub async fn generate(&self, prompt: &str) -> Result<String, AgfsError> {
        // In full implementation, this would call the OpenAI API
        // For now, return a placeholder response
        let response = format!(
            "GPT Response for prompt:\n{}\n\n(This is a placeholder - actual API integration pending)",
            prompt
        );

        Ok(response)
    }

    /// Count tokens in text (simplified estimation)
    pub fn count_tokens(text: &str) -> usize {
        // Rough estimate: ~4 characters per token
        text.len().div_ceil(4)
    }
}

impl Default for GptFS {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for GptFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        // Create a new prompt file
        let mut cache = self.cache.blocking_write();
        cache.insert(path.to_string(), GptCacheEntry {
            prompt: String::new(),
            response: String::new(),
            created_at: Utc::now(),
            tokens_used: 0,
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

        // Generate response (in a real implementation, this would call an async API)
        // For now, store the prompt and a placeholder response
        let response = format!(
            "GPT Response for: {}\n\n(Placeholder - API integration pending)",
            prompt
        );

        let mut cache = self.cache.blocking_write();
        cache.insert(path.to_string(), GptCacheEntry {
            prompt: prompt.to_string(),
            response,
            created_at: Utc::now(),
            tokens_used: Self::count_tokens(prompt) as u32,
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
                        map.insert("model".to_string(), self.model.clone());
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
                        map.insert("model".to_string(), self.model.clone());
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
                        map.insert("model".to_string(), self.model.clone());
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
    fn test_gptfs_create_and_write() {
        let fs = GptFS::new();

        fs.create("/prompt1").unwrap();
        fs.write("/prompt1", b"What is Rust?", 0, WriteFlag::NONE).unwrap();

        let data = fs.read("/prompt1", 0, -1).unwrap();
        assert!(!data.is_empty());
        let response = String::from_utf8_lossy(&data);
        assert!(response.contains("GPT Response"));
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
        fs.write("/prompt1", b"test", 0, WriteFlag::NONE).unwrap();

        let files = fs.read_dir("/").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "/prompt1");
    }

    #[test]
    fn test_gptfs_with_api_key() {
        let fs = GptFS::new().with_api_key("sk-test").with_model("gpt-4");
        assert_eq!(fs.api_key, Some("sk-test".to_string()));
        assert_eq!(fs.model, "gpt-4");
    }
}
