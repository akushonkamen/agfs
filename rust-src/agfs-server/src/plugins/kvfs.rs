//! KVFS - Key-Value Store File System
//!
//! A file system interface to a key-value store using a HashMap.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use dashmap::DashMap;
use std::io::{Cursor, Read};
use std::sync::Arc;

/// Key-value store file system
#[derive(Debug, Clone)]
pub struct Kvfs {
    /// The underlying key-value store
    store: Arc<DashMap<String, Vec<u8>>>,
}

impl Kvfs {
    /// Create a new KVFS instance
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
        }
    }

    /// Get the number of keys in the store
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Get all keys in the store
    pub fn keys(&self) -> Vec<String> {
        self.store.iter().map(|e| e.key().clone()).collect()
    }

    /// Set a key-value pair directly
    pub fn set(&self, key: &str, value: &[u8]) {
        self.store.insert(key.to_string(), value.to_vec());
    }

    /// Get a value directly
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.store.get(key).map(|v| v.clone())
    }

    /// Delete a key directly
    pub fn delete(&self, key: &str) -> bool {
        self.store.remove(key).is_some()
    }

    /// Clear all keys
    pub fn clear(&self) {
        self.store.clear()
    }
}

impl Default for Kvfs {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for Kvfs {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        let key = path.trim_start_matches('/');
        self.store.insert(key.to_string(), Vec::new());
        Ok(())
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        // KVFS doesn't have directories, but we allow mkdir for compatibility
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let key = path.trim_start_matches('/');
        if self.store.remove(key).is_some() {
            Ok(())
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        self.remove(path)
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let key = path.trim_start_matches('/');
        let data = self.store.get(key)
            .ok_or_else(|| AgfsError::not_found(path))?;

        let data = data.value();
        let offset = if offset < 0 { 0 } else { offset as usize };
        let size = if size < 0 { data.len() - offset } else { size as usize };

        if offset >= data.len() {
            return Ok(Vec::new());
        }

        let end = (offset + size).min(data.len());
        Ok(data[offset..end].to_vec())
    }

    fn write(&self, path: &str, data: &[u8], offset: i64, flags: WriteFlag) -> Result<i64, AgfsError> {
        let key = path.trim_start_matches('/');

        if flags.contains(WriteFlag::APPEND) {
            self.store.alter(key, |_, mut v| {
                v.extend_from_slice(data);
                v
            });
            return Ok(data.len() as i64);
        }

        if flags.contains(WriteFlag::TRUNCATE) || offset == 0 {
            self.store.insert(key.to_string(), data.to_vec());
            return Ok(data.len() as i64);
        }

        // Handle offset write
        let mut existing = self.store.get(key).map(|v| v.clone()).unwrap_or_default();
        let offset = if offset < 0 { existing.len() as i64 } else { offset };
        let offset = offset as usize;

        if offset >= existing.len() {
            existing.resize(offset, 0);
        }

        let start = offset;
        let end = offset + data.len();

        if end > existing.len() {
            existing.resize(end, 0);
        }

        existing[start..end].copy_from_slice(data);
        self.store.insert(key.to_string(), existing);

        Ok(data.len() as i64)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        if path != "/" && path != "" {
            return Err(AgfsError::not_found(path));
        }

        Ok(self.store.iter().map(|entry| {
            let key = entry.key();
            let value = entry.value();
            FileInfo {
                name: key.clone(),
                size: value.len() as i64,
                mode: 0o644,
                mod_time: Utc::now(),
                is_dir: false,
                is_symlink: false,
                meta: MetaData::new(),
            }
        }).collect())
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        if path == "/" || path == "" {
            return Ok(FileInfo {
                name: String::new(),
                size: self.store.len() as i64,
                mode: 0o555,
                mod_time: Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: MetaData::new(),
            });
        }

        let key = path.trim_start_matches('/');
        let entry = self.store.get(key)
            .ok_or_else(|| AgfsError::not_found(path))?;

        Ok(FileInfo {
            name: key.to_string(),
            size: entry.len() as i64,
            mode: 0o644,
            mod_time: Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: MetaData::new(),
        })
    }

    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError> {
        let old_key = old_path.trim_start_matches('/');
        let new_key = new_path.trim_start_matches('/');

        if let Some((_, value)) = self.store.remove(old_key) {
            self.store.insert(new_key.to_string(), value);
            Ok(())
        } else {
            Err(AgfsError::not_found(old_path))
        }
    }

    fn chmod(&self, _path: &str, _mode: u32) -> Result<(), AgfsError> {
        // KVFS doesn't support file permissions
        Ok(())
    }

    fn open(&self, path: &str) -> Result<Box<dyn Read + Send>, AgfsError> {
        let key = path.trim_start_matches('/');
        let data = self.store.get(key)
            .ok_or_else(|| AgfsError::not_found(path))?
            .clone();

        Ok(Box::new(Cursor::new(data)))
    }

    fn open_write(&self, _path: &str) -> Result<Box<dyn std::io::Write + Send>, AgfsError> {
        Err(AgfsError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kvfs_set_and_get() {
        let fs = Kvfs::new();
        fs.set("test", b"hello");

        let data = fs.get("test").unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn test_kvfs_read() {
        let fs = Kvfs::new();
        fs.create("/test").unwrap();
        fs.write("/test", b"hello", 0, WriteFlag::NONE).unwrap();

        let data = fs.read("/test", 0, -1).unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn test_kvfs_read_dir() {
        let fs = Kvfs::new();
        fs.set("key1", b"value1");
        fs.set("key2", b"value2");

        let files = fs.read_dir("/").unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_kvfs_delete() {
        let fs = Kvfs::new();
        fs.set("test", b"hello");
        assert!(fs.delete("test"));

        assert!(fs.get("test").is_none());
    }

    #[test]
    fn test_kvfs_len() {
        let fs = Kvfs::new();
        assert!(fs.is_empty());

        fs.set("test", b"hello");
        assert_eq!(fs.len(), 1);
    }
}
