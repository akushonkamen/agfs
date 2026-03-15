//! MemFS - In-Memory File System
//!
//! A full-featured in-memory file system implementation.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use dashmap::DashMap;
use std::path::Path;
use std::sync::Arc;

/// In-memory file data
#[derive(Debug, Clone)]
struct MemFile {
    data: Vec<u8>,
    mode: u32,
    mod_time: chrono::DateTime<chrono::Utc>,
    is_dir: bool,
}

impl MemFile {
    fn new_file() -> Self {
        Self {
            data: Vec::new(),
            mode: 0o644,
            mod_time: Utc::now(),
            is_dir: false,
        }
    }

    fn new_dir(mode: u32) -> Self {
        Self {
            data: Vec::new(),
            mode,
            mod_time: Utc::now(),
            is_dir: true,
        }
    }
}

/// In-memory file system
#[derive(Debug, Clone)]
pub struct MemFS {
    files: Arc<DashMap<String, MemFile>>,
}

impl MemFS {
    /// Create a new in-memory file system
    pub fn new() -> Self {
        Self {
            files: Arc::new(DashMap::new()),
        }
    }

    /// Normalize a path (remove trailing slashes, etc.)
    fn normalize_path(path: &str) -> String {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            String::from("/")
        } else {
            format!("/{}", path.trim_end_matches('/'))
        }
    }

    /// Get parent directory of a path
    fn parent_path(path: &str) -> Option<String> {
        if path == "/" {
            return None;
        }
        Path::new(path)
            .parent()
            .map(|p| if p.as_os_str().is_empty() { "/" } else { p.to_str().unwrap() })
            .map(String::from)
    }

    /// Get the base name of a path
    fn base_name(path: &str) -> String {
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }

    /// Ensure parent directory exists
    fn ensure_parent(&self, path: &str) -> Result<(), AgfsError> {
        if let Some(parent) = Self::parent_path(path) {
            let parent = Self::normalize_path(&parent);
            if !self.files.contains_key(&parent) {
                // Auto-create parent directory
                self.files.insert(parent.clone(), MemFile::new_dir(0o755));
            }
            if let Some(entry) = self.files.get(&parent) {
                if !entry.is_dir {
                    return Err(AgfsError::invalid_argument("parent is not a directory"));
                }
            }
        }
        Ok(())
    }
}

impl Default for MemFS {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for MemFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        let path = Self::normalize_path(path);
        self.ensure_parent(&path)?;

        if self.files.contains_key(&path) {
            return Err(AgfsError::already_exists(path));
        }

        self.files.insert(path, MemFile::new_file());
        Ok(())
    }

    fn mkdir(&self, path: &str, perm: u32) -> Result<(), AgfsError> {
        let path = Self::normalize_path(path);
        self.ensure_parent(&path)?;

        if self.files.contains_key(&path) {
            return Err(AgfsError::already_exists(path));
        }

        self.files.insert(path, MemFile::new_dir(perm));
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let path = Self::normalize_path(path);

        if !self.files.contains_key(&path) {
            return Err(AgfsError::not_found(path));
        }

        // Check if directory is empty
        if let Some(entry) = self.files.get(&path) {
            if entry.is_dir {
                // Check for children
                let prefix = format!("{}/", path.trim_end_matches('/'));
                let has_children = self.files.iter().any(|entry| {
                    let key = entry.key();
                    key != &path && key.starts_with(&prefix)
                });
                if has_children {
                    return Err(AgfsError::invalid_argument("directory not empty"));
                }
            }
        }

        self.files.remove(&path);
        Ok(())
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        let path = Self::normalize_path(path);

        if !self.files.contains_key(&path) {
            return Err(AgfsError::not_found(path));
        }

        // Remove all descendants
        let prefix = format!("{}/", path.trim_end_matches('/'));
        self.files.retain(|key, _| {
            key != &path && !key.starts_with(&prefix)
        });

        Ok(())
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let path = Self::normalize_path(path);

        let entry = self.files.get(&path)
            .ok_or_else(|| AgfsError::not_found(path))?;

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
    }

    fn write(&self, path: &str, data: &[u8], offset: i64, flags: WriteFlag) -> Result<i64, AgfsError> {
        let path = Self::normalize_path(path);

        let mut entry = self.files.get_mut(&path)
            .ok_or_else(|| AgfsError::not_found(path))?;

        if entry.is_dir {
            return Err(AgfsError::invalid_argument("is a directory"));
        }

        if flags.contains(WriteFlag::APPEND) {
            entry.data.extend_from_slice(data);
            return Ok(data.len() as i64);
        }

        if flags.contains(WriteFlag::TRUNCATE) {
            entry.data.clear();
        }

        let offset = if offset < 0 { entry.data.len() as i64 } else { offset };
        let offset = offset as usize;

        if offset >= entry.data.len() {
            entry.data.resize(offset, 0);
        }

        let start = offset;
        let end = offset + data.len();

        if end > entry.data.len() {
            entry.data.resize(end, 0);
        }

        entry.data[start..end].copy_from_slice(data);
        entry.mod_time = Utc::now();

        Ok(data.len() as i64)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        let path = Self::normalize_path(path);

        let entry = self.files.get(path.as_str())
            .ok_or_else(|| AgfsError::not_found(path.clone()))?;

        if !entry.is_dir {
            return Err(AgfsError::invalid_argument("not a directory"));
        }

        let prefix = if path == "/" {
            String::new()
        } else {
            format!("{}/", path.trim_end_matches('/'))
        };

        let mut files = Vec::new();
        for entry in self.files.iter() {
            let key = entry.key();
            if key == &path {
                continue;
            }

            // Check if this is a direct child
            if key.starts_with(&prefix) {
                let rest = &key[prefix.len()..];
                if !rest.contains('/') {
                    // Direct child
                    let file = entry.value();
                    files.push(FileInfo {
                        name: Self::base_name(key),
                        size: file.data.len() as i64,
                        mode: file.mode,
                        mod_time: file.mod_time,
                        is_dir: file.is_dir,
                        is_symlink: false,
                        meta: MetaData::default(),
                    });
                }
            }
        }

        Ok(files)
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        let path = Self::normalize_path(path);

        let entry = self.files.get(path.as_str())
            .ok_or_else(|| AgfsError::not_found(path.clone()))?;

        Ok(FileInfo {
            name: Self::base_name(&path),
            size: entry.data.len() as i64,
            mode: entry.mode,
            mod_time: entry.mod_time,
            is_dir: entry.is_dir,
            is_symlink: false,
            meta: MetaData::default(),
        })
    }

    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError> {
        let old_path = Self::normalize_path(old_path);
        let new_path = Self::normalize_path(new_path);

        if !self.files.contains_key(&old_path) {
            return Err(AgfsError::not_found(old_path));
        }

        if self.files.contains_key(&new_path) {
            return Err(AgfsError::already_exists(new_path));
        }

        self.ensure_parent(&new_path)?;

        // Rename all descendants too
        let old_prefix = format!("{}/", old_path.trim_end_matches('/'));
        let new_prefix = format!("{}/", new_path.trim_end_matches('/'));

        let mut to_move = Vec::new();
        for entry in self.files.iter() {
            let key = entry.key();
            if key == &old_path || key.starts_with(&old_prefix) {
                to_move.push((key.clone(), entry.clone()));
            }
        }

        for (old_key, file) in to_move {
            self.files.remove(&old_key);
            let new_key = if old_key == old_path {
                new_path.clone()
            } else {
                old_key.replace(&old_prefix, &new_prefix)
            };
            self.files.insert(new_key, file);
        }

        Ok(())
    }

    fn chmod(&self, path: &str, mode: u32) -> Result<(), AgfsError> {
        let path = Self::normalize_path(path);

        let mut entry = self.files.get_mut(&path)
            .ok_or_else(|| AgfsError::not_found(path))?;

        entry.mode = mode;
        Ok(())
    }

    fn open(&self, path: &str) -> Result<Box<dyn std::io::Read + Send>, AgfsError> {
        let path = Self::normalize_path(path);

        let entry = self.files.get(&path)
            .ok_or_else(|| AgfsError::not_found(path))?;

        if entry.is_dir {
            return Err(AgfsError::invalid_argument("is a directory"));
        }

        Ok(Box::new(MemReader {
            data: entry.data.clone(),
            pos: 0,
        }))
    }

    fn open_write(&self, path: &str) -> Result<Box<dyn std::io::Write + Send>, AgfsError> {
        let path = Self::normalize_path(path);

        if !self.files.contains_key(&path) {
            return Err(AgfsError::not_found(path));
        }

        Ok(Box::new(MemWriter {
            fs: self.clone(),
            path: path.to_string(),
        }))
    }
}

/// Memory reader
struct MemReader {
    data: Vec<u8>,
    pos: usize,
}

impl std::io::Read for MemReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.data.len() {
            return Ok(0);
        }
        let remaining = &self.data[self.pos..];
        let to_copy = std::cmp::min(buf.len(), remaining.len());
        buf[..to_copy].copy_from_slice(&remaining[..to_copy]);
        self.pos += to_copy;
        Ok(to_copy)
    }
}

/// Memory writer
struct MemWriter {
    fs: MemFS,
    path: String,
}

impl std::io::Write for MemWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.fs.write(&self.path, buf, -1, WriteFlag::APPEND)
            .map(|n| n as usize)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memfs_create_and_write() {
        let fs = MemFS::new();
        fs.create("/test.txt").unwrap();
        fs.write("/test.txt", b"hello", 0, WriteFlag::NONE).unwrap();

        let data = fs.read("/test.txt", 0, -1).unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn test_memfs_mkdir_and_list() {
        let fs = MemFS::new();
        fs.mkdir("/dir", 0o755).unwrap();
        fs.create("/dir/file.txt").unwrap();

        let files = fs.read_dir("/dir").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "file.txt");
    }

    #[test]
    fn test_memfs_remove() {
        let fs = MemFS::new();
        fs.create("/test.txt").unwrap();
        fs.remove("/test.txt").unwrap();

        assert!(fs.read("/test.txt", 0, -1).is_err());
    }

    #[test]
    fn test_memfs_rename() {
        let fs = MemFS::new();
        fs.create("/old.txt").unwrap();
        fs.write("/old.txt", b"test", 0, WriteFlag::NONE).unwrap();

        fs.rename("/old.txt", "/new.txt").unwrap();

        assert!(fs.read("/old.txt", 0, -1).is_err());
        let data = fs.read("/new.txt", 0, -1).unwrap();
        assert_eq!(data, b"test");
    }
}
