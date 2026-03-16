//! LocalFS - Local File System Mount
//!
//! This plugin mounts a local directory into the AGFS virtual file system.
//! Based on the Go implementation in `agfs-server/pkg/plugins/localfs/localfs.go`.

use ctxfs_sdk::{
    types::{ConfigParameter, FileInfo, MetaData, WriteFlag},
    AgfsError, FileSystem, ServicePlugin, StreamReader, Streamer, Symlinker, Truncater,
};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// Local file system implementation
#[derive(Debug, Clone)]
pub struct LocalFS {
    /// The base directory path that is mounted
    base_path: PathBuf,
    /// Plugin name for metadata
    plugin_name: String,
}

impl LocalFS {
    /// Create a new local file system
    ///
    /// # Arguments
    /// - `base_path`: The local directory to mount
    ///
    /// # Returns
    /// `Ok(LocalFS)` if the path exists and is a directory, `Err(AgfsError)` otherwise.
    pub fn new(base_path: &str) -> Result<Self, AgfsError> {
        // Resolve to absolute path
        let abs_path = fs::canonicalize(base_path).map_err(|e| {
            AgfsError::invalid_argument(format!("failed to resolve base path: {}", e))
        })?;

        // Check if base path exists and is a directory
        let metadata = fs::metadata(&abs_path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AgfsError::invalid_argument(format!("base path does not exist: {}", base_path))
            } else {
                AgfsError::internal(format!("failed to stat base path: {}", e))
            }
        })?;

        if !metadata.is_dir() {
            return Err(AgfsError::invalid_argument(format!(
                "base path is not a directory: {}",
                abs_path.display()
            )));
        }

        Ok(Self {
            base_path: abs_path,
            plugin_name: "localfs".to_string(),
        })
    }

    /// Resolve a virtual path to the actual local path
    fn resolve_path(&self, path: &str) -> PathBuf {
        // Clean the path and ensure it starts with /
        let clean_path = path.trim_start_matches('/');

        if clean_path.is_empty() {
            return self.base_path.clone();
        }

        self.base_path.join(clean_path)
    }

    /// Normalize a path to ensure consistent format
    fn normalize_path(path: &str) -> String {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", path.trim_end_matches('/'))
        }
    }
}

#[inline]
fn path_to_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}

impl FileSystem for LocalFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        let local_path = self.resolve_path(path);

        // Check if file already exists
        if local_path.exists() {
            return Err(AgfsError::already_exists(Self::normalize_path(path)));
        }

        // Check if parent directory exists
        if let Some(parent) = local_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(AgfsError::invalid_argument(format!(
                    "parent directory does not exist: {}",
                    parent.display()
                )));
            }
        }

        // Create empty file
        File::create(&local_path)
            .map_err(|e| AgfsError::internal(format!("failed to create file: {}", e)))?;

        Ok(())
    }

    fn mkdir(&self, path: &str, perm: u32) -> Result<(), AgfsError> {
        let local_path = self.resolve_path(path);

        // Check if directory already exists
        if local_path.exists() {
            return Err(AgfsError::already_exists(Self::normalize_path(path)));
        }

        // Check if parent directory exists
        if let Some(parent) = local_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(AgfsError::invalid_argument(format!(
                    "parent directory does not exist: {}",
                    parent.display()
                )));
            }
        }

        // Create directory
        fs::create_dir(&local_path)
            .map_err(|e| AgfsError::internal(format!("failed to create directory: {}", e)))?;

        // Set permissions (if supported by the platform)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = fs::set_permissions(&local_path, fs::Permissions::from_mode(perm)) {
                return Err(AgfsError::internal(format!("failed to set permissions: {}", e)));
            }
        }

        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let local_path = self.resolve_path(path);

        // Check if exists
        let metadata = local_path.metadata().map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AgfsError::not_found(Self::normalize_path(path))
            } else {
                AgfsError::internal(format!("failed to stat: {}", e))
            }
        })?;

        // If directory, check if empty
        if metadata.is_dir() {
            let entries = fs::read_dir(&local_path)
                .map_err(|e| AgfsError::internal(format!("failed to read directory: {}", e)))?;
            if entries.count() > 0 {
                return Err(AgfsError::invalid_argument("directory not empty"));
            }
        }

        // Remove file or empty directory
        fs::remove_file(&local_path)
            .or_else(|_| fs::remove_dir(&local_path))
            .map_err(|e| AgfsError::internal(format!("failed to remove: {}", e)))?;

        Ok(())
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        let local_path = self.resolve_path(path);

        // Check if exists
        if !local_path.exists() {
            return Err(AgfsError::not_found(Self::normalize_path(path)));
        }

        // Remove recursively
        fs::remove_dir_all(&local_path)
            .map_err(|e| AgfsError::internal(format!("failed to remove: {}", e)))?;

        Ok(())
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let local_path = self.resolve_path(path);

        // Check if exists and is not a directory
        let metadata = local_path.metadata().map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AgfsError::not_found(Self::normalize_path(path))
            } else {
                AgfsError::internal(format!("failed to stat: {}", e))
            }
        })?;

        if metadata.is_dir() {
            return Err(AgfsError::invalid_argument("is a directory"));
        }

        let file_size = metadata.len();

        // Handle offset
        let offset = if offset < 0 { 0 } else { offset as u64 };
        if offset >= file_size {
            return Ok(Vec::new());
        }

        // Determine read size
        let read_size = if size < 0 {
            file_size - offset
        } else {
            let size = size as u64;
            if offset + size > file_size {
                file_size - offset
            } else {
                size
            }
        };

        // Open file and read
        let mut file = File::open(&local_path)
            .map_err(|e| AgfsError::internal(format!("failed to open file: {}", e)))?;

        file.seek(SeekFrom::Start(offset))
            .map_err(|e| AgfsError::internal(format!("failed to seek: {}", e)))?;

        let mut buffer = vec![0u8; read_size as usize];
        let n = file
            .read(&mut buffer)
            .map_err(|e| AgfsError::internal(format!("failed to read: {}", e)))?;

        buffer.truncate(n);
        Ok(buffer)
    }

    fn write(
        &self,
        path: &str,
        data: &[u8],
        offset: i64,
        flags: WriteFlag,
    ) -> Result<i64, AgfsError> {
        let local_path = self.resolve_path(path);

        // Check if it's a directory
        if local_path.exists() {
            let metadata = local_path.metadata()
                .map_err(|e| AgfsError::internal(format!("failed to stat: {}", e)))?;
            if metadata.is_dir() {
                return Err(AgfsError::invalid_argument("is a directory"));
            }
        }

        // Check if parent directory exists
        if let Some(parent) = local_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(AgfsError::invalid_argument("parent directory does not exist"));
            }
        }

        // Build open flags
        let mut options = OpenOptions::new();
        options.write(true);

        if flags.contains(WriteFlag::CREATE) {
            options.create(true);
        }
        if flags.contains(WriteFlag::EXCLUSIVE) {
            options.create_new(true);
        }
        if flags.contains(WriteFlag::TRUNCATE) {
            options.truncate(true);
        }
        if flags.contains(WriteFlag::APPEND) {
            options.append(true);
        }

        // Default behavior: create and truncate (like the Go implementation)
        if flags.is_empty() && offset < 0 {
            options.create(true).truncate(true);
        } else if !flags.contains(WriteFlag::CREATE)
            && !flags.contains(WriteFlag::EXCLUSIVE)
            && !local_path.exists()
        {
            return Err(AgfsError::not_found(Self::normalize_path(path)));
        }

        let mut file = options
            .open(&local_path)
            .map_err(|e| AgfsError::internal(format!("failed to open file: {}", e)))?;

        let n = if offset >= 0 && !flags.contains(WriteFlag::APPEND) {
            // pwrite: write at specific offset
            file.seek(SeekFrom::Start(offset as u64))
                .map_err(|e| AgfsError::internal(format!("failed to seek: {}", e)))?;
            file.write(data)
                .map_err(|e| AgfsError::internal(format!("failed to write: {}", e)))?
        } else {
            // Normal write or append
            file.write(data)
                .map_err(|e| AgfsError::internal(format!("failed to write: {}", e)))?
        };

        if flags.contains(WriteFlag::SYNC) {
            file.sync_all()
                .map_err(|e| AgfsError::internal(format!("failed to sync: {}", e)))?;
        }

        Ok(n as i64)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        let local_path = self.resolve_path(path);

        // Check if directory exists
        let metadata = local_path.metadata().map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AgfsError::not_found(Self::normalize_path(path))
            } else {
                AgfsError::internal(format!("failed to stat: {}", e))
            }
        })?;

        if !metadata.is_dir() {
            return Err(AgfsError::invalid_argument("not a directory"));
        }

        // Read directory
        let entries = fs::read_dir(&local_path)
            .map_err(|e| AgfsError::internal(format!("failed to read directory: {}", e)))?;

        let mut files = Vec::new();
        for entry in entries {
            let entry = entry
                .map_err(|e| AgfsError::internal(format!("failed to read directory entry: {}", e)))?;

            let name = entry.file_name().to_string_lossy().to_string();

            let entry_metadata = entry
                .metadata()
                .map_err(|e| AgfsError::internal(format!("failed to stat entry: {}", e)))?;

            let modified = entry_metadata
                .modified()
                .ok()
                .and_then(|t| chrono::DateTime::<Utc>::from_timestamp(t_secs(&t), 0))
                .unwrap_or_else(Utc::now);

            files.push(FileInfo {
                name,
                size: entry_metadata.len() as i64,
                mode: entry_metadata.mode(),
                mod_time: modified,
                is_dir: entry_metadata.is_dir(),
                is_symlink: entry_metadata.is_symlink(),
                meta: MetaData {
                    name: self.plugin_name.clone(),
                    r#type: "local".to_string(),
                    content: {
                        let mut map = HashMap::new();
                        map.insert("local_path".to_string(), path_to_string(entry.path()));
                        map
                    },
                },
            });
        }

        Ok(files)
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        let local_path = self.resolve_path(path);

        let metadata = local_path.metadata().map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AgfsError::not_found(Self::normalize_path(path))
            } else {
                AgfsError::internal(format!("failed to stat: {}", e))
            }
        })?;

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| chrono::DateTime::<Utc>::from_timestamp(t_secs(&t), 0))
            .unwrap_or_else(Utc::now);

        let name = if path == "/" || path.is_empty() {
            "/".to_string()
        } else {
            Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string()
        };

        Ok(FileInfo {
            name,
            size: metadata.len() as i64,
            mode: metadata.mode(),
            mod_time: modified,
            is_dir: metadata.is_dir(),
            is_symlink: metadata.is_symlink(),
            meta: MetaData {
                name: self.plugin_name.clone(),
                r#type: "local".to_string(),
                content: {
                    let mut map = HashMap::new();
                    map.insert("local_path".to_string(), path_to_string(&local_path));
                    map
                },
            },
        })
    }

    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError> {
        let old_local = self.resolve_path(old_path);
        let new_local = self.resolve_path(new_path);

        // Check if old path exists
        if !old_local.exists() {
            return Err(AgfsError::not_found(Self::normalize_path(old_path)));
        }

        // Check if new path parent directory exists
        if let Some(parent) = new_local.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(AgfsError::invalid_argument("parent directory does not exist"));
            }
        }

        // Rename/move
        fs::rename(&old_local, &new_local)
            .map_err(|e| AgfsError::internal(format!("failed to rename: {}", e)))?;

        Ok(())
    }

    fn chmod(&self, path: &str, mode: u32) -> Result<(), AgfsError> {
        let local_path = self.resolve_path(path);

        // Check if exists
        if !local_path.exists() {
            return Err(AgfsError::not_found(Self::normalize_path(path)));
        }

        // Change permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = local_path
                .metadata()
                .map_err(|e| AgfsError::internal(format!("failed to get metadata: {}", e)))?
                .permissions();

            perms.set_mode(mode);
            fs::set_permissions(&local_path, perms)
                .map_err(|e| AgfsError::internal(format!("failed to chmod: {}", e)))?;
        }

        #[cfg(not(unix))]
        {
            // On non-Unix platforms, chmod is mostly a no-op
            // We still check if the file exists, which we did above
            let _ = mode;
        }

        Ok(())
    }

    fn open(&self, path: &str) -> Result<Box<dyn Read + Send>, AgfsError> {
        let local_path = self.resolve_path(path);

        let file = File::open(&local_path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AgfsError::not_found(Self::normalize_path(path))
            } else {
                AgfsError::internal(format!("failed to open file: {}", e))
            }
        })?;

        Ok(Box::new(file))
    }

    fn open_write(&self, path: &str) -> Result<Box<dyn Write + Send>, AgfsError> {
        let local_path = self.resolve_path(path);

        // Check if parent directory exists
        if let Some(parent) = local_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(AgfsError::invalid_argument("parent directory does not exist"));
            }
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&local_path)
            .map_err(|e| AgfsError::internal(format!("failed to open file for writing: {}", e)))?;

        Ok(Box::new(file))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Helper function to convert SystemTime to timestamp
#[cfg(unix)]
fn t_secs(t: &std::time::SystemTime) -> i64 {
    use std::time::UNIX_EPOCH;
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(windows)]
fn t_secs(t: &std::time::SystemTime) -> i64 {
    use std::time::UNIX_EPOCH;
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Truncater for LocalFS {
    fn truncate(&self, path: &str, size: i64) -> Result<(), AgfsError> {
        let local_path = self.resolve_path(path);

        // Check if file exists
        let metadata = local_path.metadata().map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AgfsError::not_found(Self::normalize_path(path))
            } else {
                AgfsError::internal(format!("failed to stat: {}", e))
            }
        })?;

        // Cannot truncate a directory
        if metadata.is_dir() {
            return Err(AgfsError::invalid_argument("is a directory"));
        }

        // Truncate the file
        let size = if size < 0 { 0 } else { size as u64 };
        fs::OpenOptions::new()
            .write(true)
            .open(&local_path)
            .and_then(|f| f.set_len(size))
            .map_err(|e| AgfsError::internal(format!("failed to truncate: {}", e)))?;

        Ok(())
    }
}

impl Symlinker for LocalFS {
    fn symlink(&self, target_path: &str, link_path: &str) -> Result<(), AgfsError> {
        let link_local = self.resolve_path(link_path);

        // Check if link path already exists
        if link_local.exists() || link_local.symlink_metadata().is_ok() {
            return Err(AgfsError::already_exists(Self::normalize_path(link_path)));
        }

        // Check if parent directory exists
        if let Some(parent) = link_local.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(AgfsError::invalid_argument("parent directory does not exist"));
            }
        }

        // Create symlink
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(target_path, &link_local)
                .map_err(|e| AgfsError::internal(format!("failed to create symlink: {}", e)))?;
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_file;
            if link_local.exists() {
                return Err(AgfsError::invalid_argument("target already exists"));
            }
            symlink_file(target_path, &link_local)
                .map_err(|e| AgfsError::internal(format!("failed to create symlink: {}", e)))?;
        }

        Ok(())
    }

    fn readlink(&self, link_path: &str) -> Result<String, AgfsError> {
        let link_local = self.resolve_path(link_path);

        let target = fs::read_link(&link_local).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AgfsError::not_found(Self::normalize_path(link_path))
            } else {
                AgfsError::internal(format!("failed to read symlink: {}", e))
            }
        })?;

        Ok(target.to_string_lossy().to_string())
    }
}

/// Stream reader for local files
pub struct LocalFSStreamReader {
    file: File,
    chunk_size: usize,
    eof: bool,
}

impl LocalFSStreamReader {
    pub fn new(file: File, chunk_size: usize) -> Self {
        Self {
            file,
            chunk_size,
            eof: false,
        }
    }
}

impl StreamReader for LocalFSStreamReader {
    fn read_chunk(&mut self, timeout_ms: u64) -> Result<(Vec<u8>, bool), AgfsError> {
        if self.eof {
            return Ok((Vec::new(), true));
        }

        let timeout = Duration::from_millis(timeout_ms);
        let chunk_size = self.chunk_size;

        // Use a channel to implement timeout
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let mut file_clone = self.file.try_clone().map_err(|e| {
            AgfsError::internal(format!("failed to clone file handle: {}", e))
        })?;

        std::thread::spawn(move || {
            let mut buffer = vec![0u8; chunk_size];
            let result = file_clone.read(&mut buffer).map(|n| {
                buffer.truncate(n);
                buffer
            });
            let _ = result_tx.send(result);
        });

        let buffer = result_rx
            .recv_timeout(timeout)
            .map_err(|_| AgfsError::internal("read timeout".to_string()))?
            .map_err(|e| AgfsError::internal(format!("read error: {}", e)))?;

        if buffer.is_empty() {
            self.eof = true;
            return Ok((Vec::new(), true));
        }

        // Check if this might be EOF (partial read)
        if buffer.len() < self.chunk_size {
            self.eof = true;
        }

        Ok((buffer, self.eof))
    }

    fn close(&mut self) -> Result<(), AgfsError> {
        // File will be closed when dropped
        self.eof = true;
        Ok(())
    }
}

impl Streamer for LocalFS {
    fn open_stream(&self, path: &str) -> Result<Box<dyn StreamReader>, AgfsError> {
        let local_path = self.resolve_path(path);

        // Check if file exists and is not a directory
        let metadata = local_path.metadata().map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AgfsError::not_found(Self::normalize_path(path))
            } else {
                AgfsError::internal(format!("failed to stat: {}", e))
            }
        })?;

        if metadata.is_dir() {
            return Err(AgfsError::invalid_argument("is a directory"));
        }

        let file = File::open(&local_path)
            .map_err(|e| AgfsError::internal(format!("failed to open file: {}", e)))?;

        Ok(Box::new(LocalFSStreamReader::new(file, 64 * 1024))) // 64KB chunks
    }
}

/// LocalFS plugin wrapper
pub struct LocalFSPlugin {
    fs: Option<LocalFS>,
    base_path: String,
}

impl LocalFSPlugin {
    pub fn new() -> Self {
        Self {
            fs: None,
            base_path: String::new(),
        }
    }
}

impl Default for LocalFSPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ServicePlugin for LocalFSPlugin {
    fn name(&self) -> &str {
        "localfs"
    }

    fn validate(&self, config: &HashMap<String, Value>) -> Result<(), AgfsError> {
        // Check for unknown parameters
        let allowed_keys = ["local_dir", "mount_path"];
        for key in config.keys() {
            if !allowed_keys.contains(&key.as_str()) {
                return Err(AgfsError::invalid_argument(format!(
                    "unknown parameter: {}",
                    key
                )));
            }
        }

        // Validate local_dir parameter
        let base_path = config
            .get("local_dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgfsError::invalid_argument("local_dir is required"))?;

        // Resolve to absolute path
        let abs_path = fs::canonicalize(base_path).map_err(|e| {
            AgfsError::invalid_argument(format!("failed to resolve base path: {}", e))
        })?;

        // Check if path exists and is a directory
        let metadata = fs::metadata(&abs_path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AgfsError::invalid_argument(format!(
                    "base path does not exist: {}",
                    abs_path.display()
                ))
            } else {
                AgfsError::invalid_argument(format!("failed to stat base path: {}", e))
            }
        })?;

        if !metadata.is_dir() {
            return Err(AgfsError::invalid_argument(format!(
                "base path is not a directory: {}",
                abs_path.display()
            )));
        }

        Ok(())
    }

    fn initialize(&mut self, config: HashMap<String, Value>) -> Result<(), AgfsError> {
        let base_path = config
            .get("local_dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgfsError::invalid_argument("local_dir is required"))?;

        self.base_path = base_path.to_string();

        let fs = LocalFS::new(base_path)?;
        self.fs = Some(fs);

        Ok(())
    }

    fn get_filesystem(&self) -> &dyn FileSystem {
        self.fs
            .as_ref()
            .expect("LocalFSPlugin not initialized")
    }

    fn get_readme(&self) -> &str {
        r#"LocalFS Plugin - Local File System Mount

This plugin mounts a local directory into the AGFS virtual file system.

FEATURES:
  - Mount any local directory into AGFS
  - Full POSIX file system operations
  - Direct access to local files and directories
  - Preserves file permissions and timestamps
  - Efficient file operations (no copying)

CONFIGURATION:

  Basic configuration:
  [plugins.localfs]
  enabled = true
  path = "/local"

    [plugins.localfs.config]
    local_dir = "/path/to/local/directory"

  Multiple local mounts:
  [plugins.localfs_home]
  enabled = true
  path = "/home"

    [plugins.localfs_home.config]
    local_dir = "/Users/username"

USAGE:

  List directory:
    agfs ls /local

  Read a file:
    agfs cat /local/file.txt

  Write to a file:
    agfs write /local/file.txt "Hello, World!"

  Create a directory:
    agfs mkdir /local/newdir

  Remove a file:
    agfs rm /local/file.txt

  Remove directory recursively:
    agfs rm -r /local/olddir

NOTES:
  - Changes are directly applied to the local file system
  - Be careful with rm -r as it permanently deletes files
"#
    }

    fn get_config_params(&self) -> Vec<ConfigParameter> {
        vec![ConfigParameter {
            name: "local_dir".to_string(),
            r#type: "string".to_string(),
            required: true,
            default: "".to_string(),
            description: "Local directory path to expose (must exist)".to_string(),
        }]
    }

    fn shutdown(&mut self) -> Result<(), AgfsError> {
        self.fs = None;
        Ok(())
    }
}

/// Factory function for creating LocalFS plugin instances
pub fn create_localfs_plugin() -> Box<dyn ServicePlugin> {
    Box::new(LocalFSPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_localfs_create() {
        let fs = LocalFS::new("/tmp").expect("failed to create LocalFS");
        fs.create("/test_localfs.txt").expect("failed to create file");
        fs.remove("/test_localfs.txt").expect("failed to remove file");
    }

    #[test]
    fn test_localfs_write_read() {
        let fs = LocalFS::new("/tmp").expect("failed to create LocalFS");

        // Write with create flag
        let data = b"Hello, World!";
        fs.write(
            "/test_localfs_rw.txt",
            data,
            -1,
            WriteFlag::CREATE | WriteFlag::TRUNCATE,
        )
        .expect("failed to write");

        // Read back
        let content = fs.read("/test_localfs_rw.txt", 0, -1).expect("failed to read");
        assert_eq!(content, data);

        // Cleanup
        fs.remove("/test_localfs_rw.txt").expect("failed to remove file");
    }
}
