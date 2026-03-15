//! StreamRotateFS - Rotating Stream File System
//!
//! Provides time-based rotation for streams.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData};
use chrono::{Utc, Timelike};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Rotation interval
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationInterval {
    /// Rotate every minute
    Minutely,
    /// Rotate every hour
    Hourly,
    /// Rotate every day
    Daily,
}

/// Stream data with timestamp
#[derive(Debug, Clone)]
struct TimedStreamData {
    #[allow(dead_code)]
    timestamp: chrono::DateTime<chrono::Utc>,
    data: Vec<u8>,
    #[allow(dead_code)]
    is_eof: bool,
}

/// A rotating stream
#[derive(Debug)]
struct RotatingStream {
    base_name: String,
    interval: RotationInterval,
    streams: HashMap<String, Vec<TimedStreamData>>,
}

impl RotatingStream {
    fn new(base_name: String, interval: RotationInterval) -> Self {
        Self {
            base_name,
            interval,
            streams: HashMap::new(),
        }
    }

    fn get_current_key(&self) -> String {
        let now = Utc::now();
        let key = match self.interval {
            RotationInterval::Minutely => format!("{}_{:02}{:02}",
                now.format("%Y-%m-%d"), now.hour(), now.minute()),
            RotationInterval::Hourly => format!("{}_{:02}",
                now.format("%Y-%m-%d"), now.hour()),
            RotationInterval::Daily => format!("{}",
                now.format("%Y-%m-%d")),
        };
        format!("{}/{}", self.base_name, key)
    }

    fn write(&mut self, data: &[u8]) {
        let key = self.get_current_key();
        let entry = self.streams.entry(key).or_default();
        entry.push(TimedStreamData {
            timestamp: Utc::now(),
            data: data.to_vec(),
            is_eof: false,
        });
    }

    fn list_files(&self) -> Vec<String> {
        self.streams.keys().cloned().collect()
    }

    fn read(&self, key: &str, offset: usize, size: usize) -> Vec<u8> {
        if let Some(data) = self.streams.get(key) {
            let mut result = Vec::new();
            for chunk in data.iter().skip(offset) {
                result.extend_from_slice(&chunk.data);
                if result.len() >= size {
                    break;
                }
            }
            if result.len() > size && size > 0 {
                result.truncate(size);
            }
            result
        } else {
            Vec::new()
        }
    }
}

/// Rotating stream file system
#[derive(Debug)]
pub struct StreamRotateFS {
    streams: Arc<RwLock<HashMap<String, RotatingStream>>>,
}

impl StreamRotateFS {
    /// Create a new StreamRotateFS instance
    pub fn new() -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a rotating stream
    pub fn create_stream(&self, path: &str, interval: RotationInterval) -> Result<(), AgfsError> {
        let mut streams = self.streams.write()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        streams.insert(path.to_string(), RotatingStream::new(path.to_string(), interval));
        Ok(())
    }
}

impl Default for StreamRotateFS {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for StreamRotateFS {
    fn clone(&self) -> Self {
        Self {
            streams: Arc::clone(&self.streams),
        }
    }
}

impl FileSystem for StreamRotateFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        self.create_stream(path, RotationInterval::Hourly)
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let mut streams = self.streams.write()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if streams.remove(path).is_some() {
            Ok(())
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        self.remove(path)
    }

    fn read(&self, path: &str, _offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let streams = self.streams.read()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        // Extract stream name and timestamp key
        let parts: Vec<&str> = path.rsplitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(AgfsError::not_found(path));
        }

        let stream_name = parts[1];
        let key = path;

        if let Some(stream) = streams.get(stream_name) {
            Ok(stream.read(key, 0, size as usize))
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, _flags: agfs_sdk::WriteFlag) -> Result<i64, AgfsError> {
        let mut streams = self.streams.write()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if let Some(stream) = streams.get_mut(path) {
            stream.write(data);
            Ok(data.len() as i64)
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        let streams = self.streams.read()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if path == "/" || path.is_empty() {
            // List all streams
            Ok(streams.keys().map(|name| FileInfo {
                name: name.clone(),
                size: 0,
                mode: 0o755,
                mod_time: Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: MetaData::with_type("stream"),
            }).collect())
        } else if let Some(stream) = streams.get(path) {
            // List rotated files for this stream
            Ok(stream.list_files().iter().map(|key| {
                let name = key.rsplit('/').next().unwrap_or(key).to_string();
                FileInfo {
                    name,
                    size: 0,
                    mode: 0o644,
                    mod_time: Utc::now(),
                    is_dir: false,
                    is_symlink: false,
                    meta: MetaData::default(),
                }
            }).collect())
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        let streams = self.streams.read()
            .map_err(|e| AgfsError::internal(e.to_string()))?;

        if path == "/" || path.is_empty() {
            return Ok(FileInfo {
                name: String::new(),
                size: 0,
                mode: 0o555,
                mod_time: Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: MetaData::with_type("directory"),
            });
        }

        // Check if it's a stream base
        if streams.contains_key(path) {
            return Ok(FileInfo {
                name: path.trim_start_matches('/').to_string(),
                size: 0,
                mode: 0o755,
                mod_time: Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: MetaData::with_type("stream"),
            });
        }

        // Check if it's a rotated file
        let parts: Vec<&str> = path.rsplitn(2, '/').collect();
        if parts.len() == 2 {
            if let Some(stream) = streams.get(parts[1]) {
                if stream.list_files().contains(&path.to_string()) {
                    let name = parts[0].to_string();
                    return Ok(FileInfo {
                        name,
                        size: 0,
                        mode: 0o644,
                        mod_time: Utc::now(),
                        is_dir: false,
                        is_symlink: false,
                        meta: MetaData::default(),
                    });
                }
            }
        }

        Err(AgfsError::not_found(path))
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

// StreamRotateFS doesn't implement Streamer directly as it has multiple files per stream
// Users should read from the specific rotated file paths

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streamrotatefs_create() {
        let fs = StreamRotateFS::new();
        fs.create("/test").unwrap();

        let files = fs.read_dir("/").unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_streamrotatefs_write_and_read() {
        let fs = StreamRotateFS::new();
        fs.create("/test").unwrap();

        fs.write("/test", b"hello", 0, agfs_sdk::WriteFlag::NONE).unwrap();

        let files = fs.read_dir("/test").unwrap();
        assert!(!files.is_empty());
    }
}
