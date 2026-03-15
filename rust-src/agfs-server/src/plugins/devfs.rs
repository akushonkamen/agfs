//! DevFS - Device File System
//!
//! Provides standard Unix device files like /dev/null, /dev/zero, /dev/random, /dev/urandom.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData};
use chrono::Utc;
use rand::Rng;

/// Device file system plugin
#[derive(Debug, Default, Clone)]
pub struct DevFS;

impl DevFS {
    /// Create a new DevFS instance
    pub fn new() -> Self {
        Self
    }

    /// Check if a path is a valid device
    fn is_valid_device(path: &str) -> bool {
        matches!(
            path,
            "/null" | "/zero" | "/random" | "/urandom" | "/full"
        )
    }

    /// Get device file info
    fn device_info(name: &str) -> FileInfo {
        FileInfo {
            name: name.to_string(),
            size: 0,
            mode: 0o666,
            mod_time: Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: MetaData::with_type("device"),
        }
    }
}

impl FileSystem for DevFS {
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

    fn read(&self, path: &str, _offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        if !Self::is_valid_device(path) {
            return Err(AgfsError::not_found(path));
        }

        match path {
            "/null" => {
                // /dev/null always returns EOF (empty data)
                Ok(Vec::new())
            }
            "/zero" => {
                // /dev/zero returns infinite zeros
                let count = if size < 0 { 4096 } else { size as usize };
                let data = vec![0u8; count];
                // Apply offset if specified (for /dev/zero, offset is ignored, returns zeros)
                Ok(data)
            }
            "/random" | "/urandom" => {
                // /dev/random and /dev/urandom return random bytes
                let count = if size < 0 { 4096 } else { size as usize };
                let mut rng = rand::thread_rng();
                let data: Vec<u8> = (0..count).map(|_| rng.gen()).collect();
                Ok(data)
            }
            "/full" => {
                // /dev/full always returns "No space left on device"
                Err(AgfsError::internal("No space left on device"))
            }
            _ => Err(AgfsError::not_found(path)),
        }
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, _flags: agfs_sdk::WriteFlag) -> Result<i64, AgfsError> {
        if !Self::is_valid_device(path) {
            return Err(AgfsError::not_found(path));
        }

        match path {
            "/null" => {
                // /dev/null accepts writes and discards data
                Ok(data.len() as i64)
            }
            "/zero" => {
                // /dev/zero accepts writes (and discards them)
                Ok(data.len() as i64)
            }
            "/random" | "/urandom" => {
                // These accept writes (which add entropy to the pool)
                Ok(data.len() as i64)
            }
            "/full" => {
                // /dev/full always returns "No space left on device"
                Err(AgfsError::internal("No space left on device"))
            }
            _ => Err(AgfsError::not_found(path)),
        }
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        if path != "/" {
            return Err(AgfsError::not_found(path));
        }

        Ok(vec![
            Self::device_info("null"),
            Self::device_info("zero"),
            Self::device_info("random"),
            Self::device_info("urandom"),
            Self::device_info("full"),
        ])
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

        if Self::is_valid_device(path) {
            Ok(Self::device_info(&path[1..]))
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn rename(&self, _old_path: &str, _new_path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn chmod(&self, _path: &str, _mode: u32) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn open(&self, path: &str) -> Result<Box<dyn std::io::Read + Send>, AgfsError> {
        if !Self::is_valid_device(path) {
            return Err(AgfsError::not_found(path));
        }

        match path {
            "/null" => Ok(Box::new(std::io::empty())),
            "/zero" => Ok(Box::new(std::io::repeat(0u8))),
            "/random" | "/urandom" => {
                // Create an infinite random reader
                Ok(Box::new(RandomReader))
            }
            "/full" => Err(AgfsError::internal("No space left on device")),
            _ => Err(AgfsError::not_found(path)),
        }
    }

    fn open_write(&self, path: &str) -> Result<Box<dyn std::io::Write + Send>, AgfsError> {
        if !Self::is_valid_device(path) {
            return Err(AgfsError::not_found(path));
        }

        match path {
            "/null" | "/zero" | "/random" | "/urandom" => Ok(Box::new(std::io::sink())),
            "/full" => Err(AgfsError::internal("No space left on device")),
            _ => Err(AgfsError::not_found(path)),
        }
    }
}

/// Infinite random reader for /dev/random and /dev/urandom
struct RandomReader;

impl std::io::Read for RandomReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut rng = rand::thread_rng();
        for byte in buf.iter_mut() {
            *byte = rng.gen();
        }
        Ok(buf.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devfs_null_read() {
        let fs = DevFS::new();
        let data = fs.read("/null", 0, 100).unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn test_devfs_null_write() {
        let fs = DevFS::new();
        let result = fs.write("/null", b"test", 0, agfs_sdk::WriteFlag::NONE);
        assert_eq!(result.unwrap(), 4);
    }

    #[test]
    fn test_devfs_zero_read() {
        let fs = DevFS::new();
        let data = fs.read("/zero", 0, 100).unwrap();
        assert_eq!(data.len(), 100);
        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_devfs_random_read() {
        let fs = DevFS::new();
        let data = fs.read("/random", 0, 100).unwrap();
        assert_eq!(data.len(), 100);
        // Random data should not be all zeros
        assert!(data.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_devfs_read_dir() {
        let fs = DevFS::new();
        let files = fs.read_dir("/").unwrap();
        assert_eq!(files.len(), 5);
    }

    #[test]
    fn test_devfs_stat() {
        let fs = DevFS::new();
        let info = fs.stat("/null").unwrap();
        assert_eq!(info.name, "null");
        assert!(!info.is_dir);
    }
}
