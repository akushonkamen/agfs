//! Empty plugin for testing
//!
//! This is a minimal plugin implementation used for testing the mount system.

use ctxfs_sdk::{
    types::{ConfigParameter, FileInfo},
    AgfsError, FileSystem, ServicePlugin, Symlinker, WriteFlag,
};
use serde_json::Value;
use std::collections::HashMap;

/// Empty filesystem implementation
#[derive(Debug)]
pub struct EmptyFS;

impl EmptyFS {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EmptyFS {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for EmptyFS {
    fn create(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::PermissionDenied("empty filesystem is read-only".to_string()))
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        Err(AgfsError::PermissionDenied("empty filesystem is read-only".to_string()))
    }

    fn remove(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::PermissionDenied("empty filesystem is read-only".to_string()))
    }

    fn remove_all(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::PermissionDenied("empty filesystem is read-only".to_string()))
    }

    fn read(&self, path: &str, _offset: i64, _size: i64) -> Result<Vec<u8>, AgfsError> {
        Err(AgfsError::NotFound(format!("file not found: {}", path)))
    }

    fn write(
        &self,
        _path: &str,
        _data: &[u8],
        _offset: i64,
        _flags: WriteFlag,
    ) -> Result<i64, AgfsError> {
        Err(AgfsError::PermissionDenied("empty filesystem is read-only".to_string()))
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        if path == "/" || path.is_empty() || path == "." {
            Ok(Vec::new())
        } else {
            Err(AgfsError::NotFound(format!("directory not found: {}", path)))
        }
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        if path == "/" || path.is_empty() || path == "." {
            Ok(FileInfo {
                name: "/".to_string(),
                is_dir: true,
                size: 0,
                mode: 0o755,
                mod_time: chrono::Utc::now(),
                is_symlink: false,
                meta: Default::default(),
            })
        } else {
            Err(AgfsError::NotFound(format!("file not found: {}", path)))
        }
    }

    fn rename(&self, _from: &str, _to: &str) -> Result<(), AgfsError> {
        Err(AgfsError::PermissionDenied("empty filesystem is read-only".to_string()))
    }

    fn chmod(&self, _path: &str, _mode: u32) -> Result<(), AgfsError> {
        Err(AgfsError::PermissionDenied("empty filesystem is read-only".to_string()))
    }

    fn open(&self, path: &str) -> Result<Box<dyn std::io::Read + Send>, AgfsError> {
        Err(AgfsError::NotFound(format!("file not found: {}", path)))
    }

    fn open_write(&self, _path: &str) -> Result<Box<dyn std::io::Write + Send>, AgfsError> {
        Err(AgfsError::PermissionDenied("empty filesystem is read-only".to_string()))
    }
}

impl Symlinker for EmptyFS {
    fn symlink(&self, _target: &str, _link_path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::PermissionDenied("empty filesystem is read-only".to_string()))
    }

    fn readlink(&self, _path: &str) -> Result<String, AgfsError> {
        Err(AgfsError::NotFound("not a symlink".to_string()))
    }
}

/// Empty plugin
pub struct EmptyPlugin {
    fs: EmptyFS,
}

impl EmptyPlugin {
    pub fn new() -> Self {
        Self {
            fs: EmptyFS::new(),
        }
    }
}

impl Default for EmptyPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ServicePlugin for EmptyPlugin {
    fn name(&self) -> &str {
        "empty"
    }

    fn validate(&self, _config: &HashMap<String, Value>) -> Result<(), AgfsError> {
        Ok(())
    }

    fn initialize(&mut self, _config: HashMap<String, Value>) -> Result<(), AgfsError> {
        Ok(())
    }

    fn get_filesystem(&self) -> &dyn FileSystem {
        &self.fs
    }

    fn get_readme(&self) -> &str {
        "Empty plugin - minimal read-only filesystem for testing"
    }

    fn get_config_params(&self) -> Vec<ConfigParameter> {
        vec![
            ConfigParameter {
                name: "readonly".to_string(),
                r#type: "bool".to_string(),
                required: false,
                default: "true".to_string(),
                description: "Read-only filesystem".to_string(),
            },
            ConfigParameter {
                name: "empty".to_string(),
                r#type: "bool".to_string(),
                required: false,
                default: "true".to_string(),
                description: "Empty filesystem for testing".to_string(),
            },
        ]
    }

    fn shutdown(&mut self) -> Result<(), AgfsError> {
        Ok(())
    }
}

/// Factory function for creating empty plugin instances
pub fn create_empty_plugin() -> Box<dyn ServicePlugin> {
    Box::new(EmptyPlugin::new())
}
