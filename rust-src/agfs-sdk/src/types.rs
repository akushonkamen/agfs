//! AGFS common types
//!
//! This module defines the shared data structures used throughout AGFS.
//! Based on the Go implementation in `agfs-sdk/go/types.go`.

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Structured metadata for files and directories
///
/// Corresponds to Go `MetaData` in `agfs-sdk/go/types.go`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetaData {
    /// Plugin name or identifier
    pub name: String,

    /// Type classification of the file/directory (e.g., "symlink", "stream", etc.)
    #[serde(rename = "type")]
    pub r#type: String,

    /// Additional extensible metadata as key-value pairs
    #[serde(default)]
    pub content: HashMap<String, String>,
}

impl MetaData {
    /// Create a new empty MetaData
    pub fn new() -> Self {
        Self::default()
    }

    /// Create MetaData with a type
    pub fn with_type(r#type: impl Into<String>) -> Self {
        Self {
            r#type: r#type.into(),
            ..Default::default()
        }
    }

    /// Check if this metadata indicates a symlink
    pub fn is_symlink(&self) -> bool {
        self.r#type == "symlink"
    }

    /// Add a key-value pair to content
    pub fn add_content(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.content.insert(key.into(), value.into());
        self
    }
}

/// File information
///
/// Represents file metadata similar to `os.FileInfo` in Go.
/// Corresponds to Go `FileInfo` in `agfs-sdk/go/types.go`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Base name of the file (not the full path)
    pub name: String,

    /// File size in bytes
    pub size: i64,

    /// File mode/permission bits (Unix-style)
    pub mode: u32,

    /// Last modification time
    #[serde(rename = "modTime")]
    pub mod_time: chrono::DateTime<chrono::Utc>,

    /// True if this is a directory
    #[serde(rename = "isDir")]
    pub is_dir: bool,

    /// True if this is a symbolic link
    #[serde(rename = "isSymlink")]
    pub is_symlink: bool,

    /// Structured metadata for additional information
    #[serde(default)]
    pub meta: MetaData,
}

impl FileInfo {
    /// Create a new FileInfo
    pub fn new(
        name: impl Into<String>,
        size: i64,
        mode: u32,
        mod_time: chrono::DateTime<chrono::Utc>,
        is_dir: bool,
    ) -> Self {
        Self {
            name: name.into(),
            size,
            mode,
            mod_time,
            is_dir,
            is_symlink: false,
            meta: MetaData::default(),
        }
    }

    /// Create a FileInfo for a directory
    pub fn dir(name: impl Into<String>, mode: u32) -> Self {
        Self {
            name: name.into(),
            size: 0,
            mode,
            mod_time: chrono::Utc::now(),
            is_dir: true,
            is_symlink: false,
            meta: MetaData::default(),
        }
    }

    /// Create a FileInfo for a file
    pub fn file(name: impl Into<String>, size: i64, mode: u32) -> Self {
        Self {
            name: name.into(),
            size,
            mode,
            mod_time: chrono::Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: MetaData::default(),
        }
    }

    /// Create a FileInfo for a symlink
    pub fn symlink(name: impl Into<String>, target: impl Into<String>) -> Self {
        let mut meta = MetaData::with_type("symlink");
        meta.content.insert("target".to_string(), target.into());

        Self {
            name: name.into(),
            size: 0,
            mode: 0o777,
            mod_time: chrono::Utc::now(),
            is_dir: false,
            is_symlink: true,
            meta,
        }
    }

    /// Set the metadata
    pub fn with_meta(mut self, meta: MetaData) -> Self {
        self.is_symlink = meta.is_symlink();
        self.meta = meta;
        self
    }

    /// Check if this file represents a symlink
    pub fn is_symlink(&self) -> bool {
        self.is_symlink || self.meta.is_symlink()
    }
}

bitflags! {
    /// Write flags for file write operations
    ///
    /// These flags control the behavior of write operations, similar to POSIX open flags.
    /// Corresponds to Go `WriteFlag` in `agfs-server/pkg/filesystem/filesystem.go`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct WriteFlag: u32 {
        /// Default behavior: overwrite the file
        const NONE      = 0;

        /// Append data to the end of the file (ignores offset)
        const APPEND    = 1 << 0;

        /// Create the file if it doesn't exist
        const CREATE    = 1 << 1;

        /// Fail if the file already exists (used with CREATE)
        const EXCLUSIVE = 1 << 2;

        /// Truncate the file before writing
        const TRUNCATE  = 1 << 3;

        /// Sync the file after writing (fsync)
        const SYNC      = 1 << 4;
    }
}

impl Default for WriteFlag {
    fn default() -> Self {
        Self::NONE
    }
}

// Manual serde implementation for bitflags
impl serde::Serialize for WriteFlag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.bits().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for WriteFlag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;
        Ok(Self::from_bits_retain(bits))
    }
}

bitflags! {
    /// File open flags
    ///
    /// These flags control how a file is opened, similar to POSIX O_* flags.
    /// Corresponds to Go `OpenFlag` in `agfs-server/pkg/filesystem/filesystem.go`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct OpenFlag: u32 {
        /// Read-only access
        const RDONLY   = 0;

        /// Write-only access
        const WRONLY   = 1;

        /// Read-write access
        const RDWR     = 2;

        /// Append mode
        const APPEND   = 1 << 3;

        /// Create file if it doesn't exist
        const CREATE   = 1 << 4;

        /// Exclusive creation (fail if file exists)
        const EXCL     = 1 << 5;

        /// Truncate file on open
        const TRUNC    = 1 << 6;

        /// Synchronized writes
        const SYNC     = 1 << 7;
    }
}

impl Default for OpenFlag {
    fn default() -> Self {
        Self::RDONLY
    }
}

// Manual serde implementation for bitflags
impl serde::Serialize for OpenFlag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.bits().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for OpenFlag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;
        Ok(Self::from_bits_retain(bits))
    }
}

impl OpenFlag {
    /// Check if read access is requested
    pub fn is_read(&self) -> bool {
        self.contains(Self::RDONLY) || self.contains(Self::RDWR)
    }

    /// Check if write access is requested
    pub fn is_write(&self) -> bool {
        self.contains(Self::WRONLY) || self.contains(Self::RDWR)
    }

    /// Get the access mode as an integer (0=readonly, 1=writeonly, 2=rdwr)
    pub fn access_mode(&self) -> u32 {
        if self.contains(Self::RDWR) {
            2
        } else if self.contains(Self::WRONLY) {
            1
        } else {
            0
        }
    }
}

/// Handle information
///
/// Represents an open file handle.
/// Corresponds to Go `HandleInfo` in `agfs-sdk/go/types.go`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleInfo {
    /// Unique handle ID
    #[serde(rename = "id")]
    pub id: i64,

    /// File path
    pub path: String,

    /// Open flags
    pub flags: OpenFlag,
}

/// Handle response from open operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleResponse {
    /// The assigned handle ID
    #[serde(rename = "handleId")]
    pub handle_id: i64,
}

/// Configuration parameter for plugins
///
/// Describes a configuration parameter that can be set when creating a plugin instance.
/// Corresponds to Go `ConfigParameter` in `agfs-server/pkg/plugin/plugin.go`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigParameter {
    /// Parameter name
    pub name: String,

    /// Parameter type (e.g., "string", "int", "bool")
    pub r#type: String,

    /// Whether the parameter is required
    pub required: bool,

    /// Default value (as string)
    #[serde(default)]
    pub default: String,

    /// Parameter description
    pub description: String,
}

impl ConfigParameter {
    /// Create a new ConfigParameter
    pub fn new(
        name: impl Into<String>,
        r#type: impl Into<String>,
        required: bool,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            r#type: r#type.into(),
            required,
            default: String::new(),
            description: description.into(),
        }
    }

    /// Set a default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = default.into();
        self
    }
}

/// Symlink request for API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymlinkRequest {
    /// Target path the symlink points to
    pub target: String,
}

/// Readlink response from API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadlinkResponse {
    /// Target path the symlink points to
    pub target: String,
}

/// Rename request for API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameRequest {
    /// New path for the file/directory
    #[serde(rename = "newPath")]
    pub new_path: String,
}

/// Chmod request for API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChmodRequest {
    /// Permission mode to set
    pub mode: u32,
}

/// Directory listing response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    /// List of files in the directory
    pub files: Vec<FileInfo>,
}

/// Success response from API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    /// Success message
    pub message: String,
}

/// Error response from API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
}

/// Grep search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepRequest {
    /// Path to search in
    pub path: String,

    /// Regex pattern to search for
    pub pattern: String,

    /// Whether to search recursively
    #[serde(default)]
    pub recursive: bool,

    /// Case-insensitive search
    #[serde(rename = "caseInsensitive")]
    #[serde(default)]
    pub case_insensitive: bool,
}

/// A single grep match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    /// File where the match was found
    pub file: String,

    /// Line number of the match
    pub line: i32,

    /// Line content containing the match
    pub content: String,
}

/// Grep search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepResponse {
    /// All matches found
    pub matches: Vec<GrepMatch>,

    /// Total count of matches
    pub count: i32,
}

/// Digest calculation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestRequest {
    /// Hash algorithm to use ("xxh3" or "md5")
    pub algorithm: String,

    /// Path to the file
    pub path: String,
}

/// Digest calculation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestResponse {
    /// Algorithm used
    pub algorithm: String,

    /// File path
    pub path: String,

    /// Hex-encoded digest
    pub digest: String,
}

/// Server capabilities response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesResponse {
    /// Server version
    pub version: String,

    /// Supported features
    pub features: Vec<String>,
}
