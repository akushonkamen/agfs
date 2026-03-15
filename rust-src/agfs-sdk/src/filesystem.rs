//! FileSystem trait and related traits
//!
//! This module defines the core abstraction for file system implementations in AGFS.
//! Based on the Go implementation in `agfs-server/pkg/filesystem/filesystem.go`.

use crate::error::AgfsError;
use crate::types::{FileInfo, WriteFlag};
use std::io::{Read, Write};

/// Core file system trait
///
/// This trait defines the interface that all file system plugins must implement.
/// It corresponds to the Go `filesystem.FileSystem` interface.
///
/// # Thread Safety
/// All implementations must be thread-safe (`Send + Sync`).
///
/// # Blocking Behavior
/// The trait methods are synchronous (blocking), matching the Go implementation.
/// For async operations, use the runtime handle to bridge within implementations.
pub trait FileSystem: Send + Sync {
    /// Create a new empty file
    ///
    /// Creates a new file at the specified path.
    /// Returns an error if the file already exists or if the path is invalid.
    fn create(&self, path: &str) -> Result<(), AgfsError>;

    /// Create a new directory
    ///
    /// Creates a directory at the specified path with the given permissions.
    /// The `perm` parameter uses Unix-style permission bits (e.g., 0o755).
    fn mkdir(&self, path: &str, perm: u32) -> Result<(), AgfsError>;

    /// Remove a file or empty directory
    ///
    /// Removes the file or directory at the specified path.
    /// Returns an error if the path doesn't exist or if the directory is not empty.
    fn remove(&self, path: &str) -> Result<(), AgfsError>;

    /// Remove a path and all its children
    ///
    /// Recursively removes a file or directory tree.
    /// Returns an error if the path doesn't exist.
    fn remove_all(&self, path: &str) -> Result<(), AgfsError>;

    /// Read file content
    ///
    /// Reads up to `size` bytes from the file starting at `offset`.
    ///
    /// # Arguments
    /// - `path`: The file path to read from
    /// - `offset`: Starting position in bytes (0 for beginning)
    /// - `size`: Number of bytes to read (-1 or value larger than file size means read all)
    ///
    /// # Returns
    /// The bytes read. May return fewer bytes than requested if near EOF.
    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError>;

    /// Write data to a file
    ///
    /// Writes data to a file at the specified offset with the given flags.
    ///
    /// # Arguments
    /// - `path`: The file path to write to
    /// - `data`: The data bytes to write
    /// - `offset`: Write position in bytes (-1 means append or overwrite depending on flags)
    /// - `flags`: Write flags controlling behavior (append, create, truncate, sync)
    ///
    /// # Returns
    /// The number of bytes written.
    fn write(&self, path: &str, data: &[u8], offset: i64, flags: WriteFlag) -> Result<i64, AgfsError>;

    /// List directory contents
    ///
    /// Returns a list of file info entries for the directory at the specified path.
    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError>;

    /// Get file information
    ///
    /// Returns metadata about the file or directory at the specified path.
    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError>;

    /// Rename/move a file or directory
    ///
    /// Moves a file or directory from `old_path` to `new_path`.
    /// Returns an error if either path is invalid.
    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError>;

    /// Change file permissions
    ///
    /// Changes the permission bits of the file at the specified path.
    fn chmod(&self, path: &str, mode: u32) -> Result<(), AgfsError>;

    /// Open a file for reading
    ///
    /// Returns a boxed reader for the file at the specified path.
    /// The reader is owned and will be closed when dropped.
    fn open(&self, path: &str) -> Result<Box<dyn Read + Send>, AgfsError>;

    /// Open a file for writing
    ///
    /// Returns a boxed writer for the file at the specified path.
    /// The writer is owned and will flush/close when dropped.
    fn open_write(&self, path: &str) -> Result<Box<dyn Write + Send>, AgfsError>;

    /// Get `Any` reference for downcasting
    ///
    /// This enables downcasting from a `FileSystem` trait object to its concrete type,
    /// which is necessary for checking if a filesystem implements extension traits
    /// like `Streamer`, `Toucher`, `Symlinker`, or `Truncater`.
    ///
    /// Default implementation returns a reference that cannot be downcast.
    /// Concrete types should override this to return `self`.
    fn as_any(&self) -> &dyn std::any::Any {
        &()
    }
}

/// Stream reader trait
///
/// This trait is implemented by file systems that support streaming reads.
/// It allows for chunked reading with timeout support, used by streamfs for real-time data.
pub trait StreamReader: Send + Sync {
    /// Read the next chunk of data with a timeout
    ///
    /// # Arguments
    /// - `timeout_ms`: Timeout in milliseconds
    ///
    /// # Returns
    /// A tuple of (data, is_eof, error):
    /// - `data`: The chunk data (may be empty if timeout or EOF)
    /// - `is_eof`: True if the stream is closed/ended
    /// - `error`: None on success, Some(AgfsError) on failure
    fn read_chunk(&mut self, timeout_ms: u64) -> Result<(Vec<u8>, bool), AgfsError>;

    /// Close the stream and release resources
    fn close(&mut self) -> Result<(), AgfsError>;
}

/// Streamer extension trait
///
/// This trait is implemented by file systems that support streaming reads.
/// It allows multiple readers to consume data in real-time as it's written (fanout/broadcast).
///
/// Used by: streamfs, streamrotatefs
pub trait Streamer: FileSystem {
    /// Open a stream for reading
    ///
    /// Returns a StreamReader that can read chunks progressively.
    /// Multiple readers can open the same stream for fanout scenarios.
    fn open_stream(&self, path: &str) -> Result<Box<dyn StreamReader>, AgfsError>;
}

/// Toucher extension trait
///
/// This trait is implemented by file systems that support efficient touch operations.
///
/// Used by: heartbeatfs
pub trait Toucher: FileSystem {
    /// Update the modification time of a file
    ///
    /// If the file doesn't exist, it should be created as an empty file.
    fn touch(&self, path: &str) -> Result<(), AgfsError>;
}

/// Symlinker extension trait
///
/// This trait is implemented by file systems that support symbolic links.
pub trait Symlinker: FileSystem {
    /// Create a symbolic link
    ///
    /// Creates a symbolic link at `link_path` pointing to `target_path`.
    /// The target path can be relative or absolute, and doesn't need to exist.
    fn symlink(&self, target: &str, link: &str) -> Result<(), AgfsError>;

    /// Read the target of a symbolic link
    ///
    /// Returns the target path that the symbolic link at `link` points to.
    fn readlink(&self, link: &str) -> Result<String, AgfsError>;
}

/// Truncater extension trait
///
/// This trait is implemented by file systems that support file truncation.
pub trait Truncater: FileSystem {
    /// Truncate a file to the specified size
    ///
    /// - If `size` is 0, the file content is cleared
    /// - If `size` is smaller than current file size, content is truncated
    /// - If `size` is larger than current file size, file is extended with zeros
    fn truncate(&self, path: &str, size: i64) -> Result<(), AgfsError>;
}
