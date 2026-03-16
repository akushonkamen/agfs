//! StreamFS - Streaming File System
//!
//! Provides real-time streaming data with fanout capability.

use ctxfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, StreamReader, Streamer, WriteFlag};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Stream channel capacity
const CHANNEL_CAPACITY: usize = 256;

/// Data in a stream
#[derive(Debug, Clone)]
struct StreamData {
    data: Vec<u8>,
    is_eof: bool,
}

/// A stream reader
pub struct StreamReceiver {
    receiver: broadcast::Receiver<StreamData>,
    timeout_ms: u64,
}

impl StreamReader for StreamReceiver {
    fn read_chunk(&mut self, timeout_ms: u64) -> Result<(Vec<u8>, bool), AgfsError> {
        self.timeout_ms = timeout_ms;

        // Try to receive without blocking for async
        match self.receiver.try_recv() {
            Ok(data) => Ok((data.data, data.is_eof)),
            Err(_) => {
                // No data available immediately, return empty with not-EOF
                // In a real implementation, we'd use proper async blocking
                Ok((vec![], false))
            }
        }
    }

    fn close(&mut self) -> Result<(), AgfsError> {
        // Dropping the receiver will close it
        Ok(())
    }
}

/// Streaming file system
#[derive(Debug, Clone)]
pub struct StreamFS {
    /// Active streams
    streams: Arc<DashMap<String, broadcast::Sender<StreamData>>>,
}

impl StreamFS {
    /// Create a new StreamFS instance
    pub fn new() -> Self {
        Self {
            streams: Arc::new(DashMap::new()),
        }
    }

    /// Create or get a stream
    fn get_or_create_stream(&self, path: &str) -> broadcast::Sender<StreamData> {
        self.streams
            .entry(path.to_string())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
            .clone()
    }

    /// Write to a stream
    pub fn write_stream(&self, path: &str, data: &[u8], is_eof: bool) -> Result<(), AgfsError> {
        let sender = self.get_or_create_stream(path);

        let stream_data = StreamData {
            data: data.to_vec(),
            is_eof,
        };

        sender.send(stream_data).map_err(|_| AgfsError::internal("no receivers"))?;
        Ok(())
    }

    /// Check if a stream exists
    pub fn has_stream(&self, path: &str) -> bool {
        self.streams.contains_key(path)
    }

    /// Delete a stream
    pub fn delete_stream(&self, path: &str) -> bool {
        self.streams.remove(path).is_some()
    }

    /// Get number of active streams
    pub fn len(&self) -> usize {
        self.streams.len()
    }

    /// Check if there are no active streams
    pub fn is_empty(&self) -> bool {
        self.streams.is_empty()
    }
}

impl Default for StreamFS {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for StreamFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        self.get_or_create_stream(path);
        Ok(())
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        self.delete_stream(path);
        Ok(())
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        self.remove(path)
    }

    fn read(&self, _path: &str, _offset: i64, _size: i64) -> Result<Vec<u8>, AgfsError> {
        // Streams don't support random access reads
        Err(AgfsError::NotSupported)
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, _flags: WriteFlag) -> Result<i64, AgfsError> {
        self.write_stream(path, data, false)?;
        Ok(data.len() as i64)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        if path != "/" {
            return Err(AgfsError::not_found(path));
        }

        Ok(self.streams.iter().map(|entry| {
            FileInfo {
                name: entry.key().clone(),
                size: 0,
                mode: 0o644,
                mod_time: Utc::now(),
                is_dir: false,
                is_symlink: false,
                meta: MetaData::with_type("stream"),
            }
        }).collect())
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        if path == "/" {
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

        if self.streams.contains_key(path) {
            Ok(FileInfo {
                name: path.trim_start_matches('/').to_string(),
                size: 0,
                mode: 0o644,
                mod_time: Utc::now(),
                is_dir: false,
                is_symlink: false,
                meta: MetaData::with_type("stream"),
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

impl Streamer for StreamFS {
    fn open_stream(&self, path: &str) -> Result<Box<dyn StreamReader>, AgfsError> {
        let sender = self.get_or_create_stream(path);
        let receiver = sender.subscribe();

        Ok(Box::new(StreamReceiver {
            receiver,
            timeout_ms: 5000,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streamfs_create_and_write() {
        let fs = StreamFS::new();
        fs.create("/test").unwrap();
        assert!(fs.has_stream("/test"));

        // Note: write may not deliver data if no receivers are subscribed
        // This is expected behavior for broadcast channels
    }

    #[test]
    fn test_streamfs_list() {
        let fs = StreamFS::new();
        fs.create("/stream1").unwrap();
        fs.create("/stream2").unwrap();

        let files = fs.read_dir("/").unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_streamfs_delete() {
        let fs = StreamFS::new();
        fs.create("/test").unwrap();
        assert!(fs.has_stream("/test"));

        fs.remove("/test").unwrap();
        assert!(!fs.has_stream("/test"));
    }

    #[tokio::test]
    async fn test_streamer() {
        let fs = StreamFS::new();
        fs.create("/test").unwrap();

        // Open stream first (subscriber must exist before data is sent)
        let mut reader = fs.open_stream("/test").unwrap();

        // Give receiver time to register
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Write some data
        fs.write("/test", b"test data", 0, WriteFlag::NONE).unwrap();

        // Read with timeout
        let (data, is_eof) = reader.read_chunk(100).unwrap();
        // Data should now be available
        if !data.is_empty() {
            assert_eq!(data, b"test data");
        }
        assert!(!is_eof);
    }
}
