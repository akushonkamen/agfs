//! HelloFS - A simple example plugin
//!
//! This plugin provides a read-only file system with a single file that returns "Hello, World!".

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData};
use chrono::Utc;
use std::io::{Cursor, Read};

/// Hello file system - a simple example plugin
#[derive(Debug, Default, Clone)]
pub struct HelloFS;

impl HelloFS {
    /// Create a new HelloFS instance
    pub fn new() -> Self {
        Self
    }

    /// Get the hello message
    fn hello_message() -> &'static str {
        "Hello, World!\n"
    }
}

impl FileSystem for HelloFS {
    fn create(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn remove(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn remove_all(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn read(&self, path: &str, _offset: i64, _size: i64) -> Result<Vec<u8>, AgfsError> {
        match path {
            "/" | "" => Err(AgfsError::invalid_argument("is a directory")),
            "/hello" => Ok(Self::hello_message().as_bytes().to_vec()),
            _ => Err(AgfsError::not_found(path)),
        }
    }

    fn write(&self, _path: &str, _data: &[u8], _offset: i64, _flags: agfs_sdk::WriteFlag) -> Result<i64, AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        if path != "/" && path != "" {
            return Err(AgfsError::not_found(path));
        }

        Ok(vec![FileInfo {
            name: "hello".to_string(),
            size: Self::hello_message().len() as i64,
            mode: 0o444,
            mod_time: Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: MetaData::with_type("file"),
        }])
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        match path {
            "/" | "" => Ok(FileInfo {
                name: String::new(),
                size: 0,
                mode: 0o555,
                mod_time: Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: MetaData::with_type("directory"),
            }),
            "/hello" => Ok(FileInfo {
                name: "hello".to_string(),
                size: Self::hello_message().len() as i64,
                mode: 0o444,
                mod_time: Utc::now(),
                is_dir: false,
                is_symlink: false,
                meta: MetaData::with_type("file"),
            }),
            _ => Err(AgfsError::not_found(path)),
        }
    }

    fn rename(&self, _old_path: &str, _new_path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn chmod(&self, _path: &str, _mode: u32) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn open(&self, path: &str) -> Result<Box<dyn Read + Send>, AgfsError> {
        match path {
            "/hello" => Ok(Box::new(Cursor::new(Self::hello_message().as_bytes().to_vec()))),
            _ => Err(AgfsError::not_found(path)),
        }
    }

    fn open_write(&self, _path: &str) -> Result<Box<dyn std::io::Write + Send>, AgfsError> {
        Err(AgfsError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hellofs_read() {
        let fs = HelloFS::new();
        let data = fs.read("/hello", 0, -1).unwrap();
        assert_eq!(data, b"Hello, World!\n");
    }

    #[test]
    fn test_hellofs_read_dir() {
        let fs = HelloFS::new();
        let files = fs.read_dir("/").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "hello");
    }

    #[test]
    fn test_hellofs_stat() {
        let fs = HelloFS::new();
        let info = fs.stat("/hello").unwrap();
        assert_eq!(info.name, "hello");
        assert!(!info.is_dir);
    }
}
