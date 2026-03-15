//! SqlFS - SQL Database File System
//!
//! Stores files as BLOBs in a SQL database.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory file storage (placeholder for SQL backend)
#[derive(Debug, Clone)]
struct FileEntry {
    data: Vec<u8>,
    mode: u32,
    mod_time: chrono::DateTime<chrono::Utc>,
    is_dir: bool,
}

/// SQL file system
#[derive(Debug, Clone)]
pub struct SqlFS {
    files: Arc<RwLock<HashMap<String, FileEntry>>>,
}

impl SqlFS {
    /// Create a new SqlFS instance
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize database connection
    pub async fn initialize(&self, _connection_string: &str) -> Result<(), AgfsError> {
        // In full implementation, would establish SQL connection and create tables
        Ok(())
    }
}

impl Default for SqlFS {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for SqlFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        let mut files = self.files.write()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if files.contains_key(path) {
            return Err(AgfsError::already_exists(path));
        }

        files.insert(path.to_string(), FileEntry {
            data: Vec::new(),
            mode: 0o644,
            mod_time: Utc::now(),
            is_dir: false,
        });

        Ok(())
    }

    fn mkdir(&self, path: &str, perm: u32) -> Result<(), AgfsError> {
        let mut files = self.files.write()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        files.insert(path.to_string(), FileEntry {
            data: Vec::new(),
            mode: perm,
            mod_time: Utc::now(),
            is_dir: true,
        });

        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let mut files = self.files.write()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if files.remove(path).is_some() {
            Ok(())
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        let mut files = self.files.write()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        files.retain(|k, _| !k.starts_with(path));

        Ok(())
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let files = self.files.read()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if let Some(entry) = files.get(path) {
            if entry.is_dir {
                return Err(AgfsError::invalid_argument("is a directory"));
            }

            let data = &entry.data;
            let offset = if offset < 0 { 0 } else { offset as usize };
            let size = if size < 0 { data.len() - offset } else { size as usize };

            if offset >= data.len() {
                return Ok(Vec::new());
            }

            let end = (offset + size).min(data.len());
            Ok(data[offset..end].to_vec())
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, _flags: WriteFlag) -> Result<i64, AgfsError> {
        let mut files = self.files.write()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if let Some(entry) = files.get_mut(path) {
            entry.data = data.to_vec();
            entry.mod_time = Utc::now();
            Ok(data.len() as i64)
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        let files = self.files.read()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if path == "/" || path.is_empty() {
            // List all root files
            return Ok(files
                .iter()
                .filter(|(_, v)| !v.is_dir)
                .map(|(k, v)| FileInfo {
                    name: k.clone(),
                    size: v.data.len() as i64,
                    mode: v.mode,
                    mod_time: v.mod_time,
                    is_dir: false,
                    is_symlink: false,
                    meta: MetaData::default(),
                })
                .collect());
        }

        Ok(Vec::new())
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        if path == "/" || path.is_empty() {
            return Ok(FileInfo {
                name: String::new(),
                size: 0,
                mode: 0o555,
                mod_time: Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: MetaData::default(),
            });
        }

        let files = self.files.read()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if let Some(entry) = files.get(path) {
            Ok(FileInfo {
                name: path.trim_start_matches('/').to_string(),
                size: entry.data.len() as i64,
                mode: entry.mode,
                mod_time: entry.mod_time,
                is_dir: entry.is_dir,
                is_symlink: false,
                meta: MetaData::default(),
            })
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError> {
        let mut files = self.files.write()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if let Some(entry) = files.remove(old_path) {
            files.insert(new_path.to_string(), entry);
            Ok(())
        } else {
            Err(AgfsError::not_found(old_path))
        }
    }

    fn chmod(&self, path: &str, mode: u32) -> Result<(), AgfsError> {
        let mut files = self.files.write()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if let Some(entry) = files.get_mut(path) {
            entry.mode = mode;
            Ok(())
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn open(&self, path: &str) -> Result<Box<dyn std::io::Read + Send>, AgfsError> {
        let files = self.files.read()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if let Some(entry) = files.get(path) {
            if entry.is_dir {
                return Err(AgfsError::invalid_argument("is a directory"));
            }
            Ok(Box::new(std::io::Cursor::new(entry.data.clone())))
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn open_write(&self, _path: &str) -> Result<Box<dyn std::io::Write + Send>, AgfsError> {
        Err(AgfsError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlfs_create_and_write() {
        let fs = SqlFS::new();
        fs.initialize("sqlite::memory:").await.unwrap();

        fs.create("/test.txt").unwrap();
        fs.write("/test.txt", b"hello from sqlfs", 0, WriteFlag::NONE).unwrap();

        let data = fs.read("/test.txt", 0, -1).unwrap();
        assert_eq!(data, b"hello from sqlfs");
    }
}
