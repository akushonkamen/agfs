//! QueueFS - Queue File System
//!
//! Provides message queue functionality with SQLite backend.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Queue item
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct QueueItem {
    id: u64,
    data: Vec<u8>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// In-memory queue
#[derive(Debug)]
struct Queue {
    items: Vec<QueueItem>,
    next_id: u64,
}

impl Queue {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
        }
    }

    fn enqueue(&mut self, data: &[u8]) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        self.items.push(QueueItem {
            id,
            data: data.to_vec(),
            created_at: Utc::now(),
        });

        id
    }

    fn dequeue(&mut self) -> Option<Vec<u8>> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.items.remove(0).data)
        }
    }

    fn peek(&self) -> Option<Vec<u8>> {
        self.items.first().map(|item| item.data.clone())
    }

    fn size(&self) -> usize {
        self.items.len()
    }

    fn clear(&mut self) {
        self.items.clear();
    }

    fn list(&self) -> Vec<QueueItem> {
        self.items.clone()
    }
}

/// Queue file system
#[derive(Debug, Clone)]
pub struct QueueFS {
    queues: Arc<DashMap<String, Arc<Mutex<Queue>>>>,
}

impl QueueFS {
    /// Create a new QueueFS instance
    pub fn new() -> Self {
        Self {
            queues: Arc::new(DashMap::new()),
        }
    }

    /// Get or create a queue
    fn get_or_create_queue(&self, path: &str) -> Arc<Mutex<Queue>> {
        self.queues
            .entry(path.trim_start_matches('/').to_string())
            .or_insert_with(|| Arc::new(Mutex::new(Queue::new())))
            .clone()
    }

    /// Enqueue data
    pub async fn enqueue(&self, path: &str, data: &[u8]) -> Result<u64, AgfsError> {
        let queue = self.get_or_create_queue(path);
        let mut queue = queue.lock().await;
        Ok(queue.enqueue(data))
    }

    /// Dequeue data
    pub async fn dequeue(&self, path: &str) -> Result<Option<Vec<u8>>, AgfsError> {
        let queue = self.get_or_create_queue(path);
        let mut queue = queue.lock().await;
        Ok(queue.dequeue())
    }

    /// Peek at queue
    pub async fn peek(&self, path: &str) -> Result<Option<Vec<u8>>, AgfsError> {
        let queue = self.get_or_create_queue(path);
        let queue = queue.lock().await;
        Ok(queue.peek())
    }

    /// Get queue size
    pub async fn size(&self, path: &str) -> Result<usize, AgfsError> {
        let queue = self.get_or_create_queue(path);
        let queue = queue.lock().await;
        Ok(queue.size())
    }

    /// Clear queue
    pub async fn clear(&self, path: &str) -> Result<(), AgfsError> {
        let queue = self.get_or_create_queue(path);
        let mut queue = queue.lock().await;
        queue.clear();
        Ok(())
    }

    /// List all queue items
    pub async fn list(&self, path: &str) -> Result<Vec<QueueItem>, AgfsError> {
        let queue = self.get_or_create_queue(path);
        let queue = queue.lock().await;
        Ok(queue.list())
    }

    /// Delete a queue
    pub fn delete_queue(&self, path: &str) -> bool {
        let key = path.trim_start_matches('/');
        self.queues.remove(key).is_some()
    }

    /// Get queue count
    pub fn queue_count(&self) -> usize {
        self.queues.len()
    }
}

impl Default for QueueFS {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for QueueFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        // Creating a queue is implicit, just get_or_create
        self.get_or_create_queue(path);
        Ok(())
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        if self.delete_queue(path) {
            Ok(())
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        self.remove(path)
    }

    fn read(&self, path: &str, _offset: i64, _size: i64) -> Result<Vec<u8>, AgfsError> {
        // For queues, read is equivalent to dequeue
        let queue = self.get_or_create_queue(path);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let mut queue = queue.lock().await;
                Ok(queue.dequeue().unwrap_or_default())
            })
        })
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, _flags: WriteFlag) -> Result<i64, AgfsError> {
        let queue = self.get_or_create_queue(path);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let mut queue = queue.lock().await;
                queue.enqueue(data);
                Ok(data.len() as i64)
            })
        })
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        if path != "/" && !path.is_empty() {
            return Err(AgfsError::not_found(path));
        }

        Ok(self.queues.iter().map(|entry| {
            FileInfo {
                name: entry.key().clone(),
                size: 0,
                mode: 0o755,
                mod_time: Utc::now(),
                is_dir: false,
                is_symlink: false,
                meta: MetaData::with_type("queue"),
            }
        }).collect())
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        if path == "/" || path.is_empty() {
            return Ok(FileInfo {
                name: String::new(),
                size: self.queues.len() as i64,
                mode: 0o555,
                mod_time: Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: MetaData::with_type("directory"),
            });
        }

        let key = path.trim_start_matches('/');
        if self.queues.contains_key(key) {
            Ok(FileInfo {
                name: key.to_string(),
                size: 0,
                mode: 0o644,
                mod_time: Utc::now(),
                is_dir: false,
                is_symlink: false,
                meta: MetaData::with_type("queue"),
            })
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn rename(&self, _old_path: &str, _new_path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn chmod(&self, _path: &str, _mode: u32) -> Result<(), AgfsError> {
        Ok(())
    }

    fn open(&self, _path: &str) -> Result<Box<dyn std::io::Read + Send>, AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn open_write(&self, _path: &str) -> Result<Box<dyn std::io::Write + Send>, AgfsError> {
        Err(AgfsError::NotSupported)
    }
}

/// QueueFS plugin wrapper
pub struct QueueFSPlugin {
    fs: QueueFS,
}

impl QueueFSPlugin {
    pub fn new() -> Self {
        Self { fs: QueueFS::new() }
    }
}

impl Default for QueueFSPlugin {
    fn default() -> Self {
        Self::new()
    }
}

use agfs_sdk::{types::ConfigParameter, ServicePlugin};
use std::collections::HashMap;

impl ServicePlugin for QueueFSPlugin {
    fn name(&self) -> &str {
        "queuefs"
    }

    fn validate(&self, _config: &HashMap<String, serde_json::Value>) -> Result<(), AgfsError> {
        Ok(())
    }

    fn initialize(&mut self, _config: HashMap<String, serde_json::Value>) -> Result<(), AgfsError> {
        Ok(())
    }

    fn get_filesystem(&self) -> &dyn FileSystem {
        &self.fs
    }

    fn get_readme(&self) -> &str {
        "QueueFS - Message queue file system with in-memory storage"
    }

    fn get_config_params(&self) -> Vec<ConfigParameter> {
        vec![]
    }

    fn shutdown(&mut self) -> Result<(), AgfsError> {
        self.fs.queues.clear();
        Ok(())
    }
}

/// Factory function for creating queuefs plugin instances
pub fn create_queuefs_plugin() -> Box<dyn ServicePlugin> {
    Box::new(QueueFSPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queuefs_enqueue_dequeue() {
        let fs = QueueFS::new();
        fs.create("/test").unwrap();

        fs.enqueue("/test", b"hello").await.unwrap();
        fs.enqueue("/test", b"world").await.unwrap();

        let data = fs.dequeue("/test").await.unwrap().unwrap();
        assert_eq!(data, b"hello");

        let data = fs.dequeue("/test").await.unwrap().unwrap();
        assert_eq!(data, b"world");

        let data = fs.dequeue("/test").await.unwrap();
        assert!(data.is_none());
    }

    #[tokio::test]
    async fn test_queuefs_size() {
        let fs = QueueFS::new();

        fs.enqueue("/test", b"hello").await.unwrap();
        fs.enqueue("/test", b"world").await.unwrap();

        let size = fs.size("/test").await.unwrap();
        assert_eq!(size, 2);
    }

    #[tokio::test]
    async fn test_queuefs_peek() {
        let fs = QueueFS::new();

        fs.enqueue("/test", b"hello").await.unwrap();

        let data = fs.peek("/test").await.unwrap().unwrap();
        assert_eq!(data, b"hello");

        // Peek doesn't remove
        let size = fs.size("/test").await.unwrap();
        assert_eq!(size, 1);
    }

    #[tokio::test]
    async fn test_queuefs_clear() {
        let fs = QueueFS::new();

        fs.enqueue("/test", b"hello").await.unwrap();
        fs.enqueue("/test", b"world").await.unwrap();

        fs.clear("/test").await.unwrap();

        let size = fs.size("/test").await.unwrap();
        assert_eq!(size, 0);
    }

    #[test]
    fn test_queuefs_list() {
        let fs = QueueFS::new();
        fs.create("/queue1").unwrap();
        fs.create("/queue2").unwrap();

        let files = fs.read_dir("/").unwrap();
        assert_eq!(files.len(), 2);
    }
}
