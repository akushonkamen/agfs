//! Common response types for AGFS API

use serde::{Deserialize, Serialize};

/// Success response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    /// Success message
    pub message: String,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
}

/// File info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfoResponse {
    /// File name
    pub name: String,
    /// File size in bytes
    pub size: i64,
    /// File mode/permissions
    pub mode: u32,
    /// Last modification time (RFC3339)
    #[serde(rename = "modTime")]
    pub mod_time: String,
    /// True if this is a directory
    #[serde(rename = "isDir")]
    pub is_dir: bool,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Directory listing response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    /// List of files in the directory
    pub files: Vec<FileInfoResponse>,
}

/// Rename request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameRequest {
    /// New path for the file/directory
    #[serde(rename = "newPath")]
    pub new_path: String,
}

/// Chmod request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChmodRequest {
    /// Permission mode to set
    pub mode: u32,
}

/// Symlink request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymlinkRequest {
    /// Target path the symlink points to
    pub target: String,
}

/// Readlink response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadlinkResponse {
    /// Target path the symlink points to
    pub target: String,
}

/// Digest request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestRequest {
    /// Hash algorithm ("xxh3" or "md5")
    pub algorithm: String,
    /// Path to the file
    pub path: String,
}

/// Digest response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestResponse {
    /// Algorithm used
    pub algorithm: String,
    /// File path
    pub path: String,
    /// Hex-encoded digest
    pub digest: String,
}

/// Grep request
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
    /// Stream results as NDJSON
    #[serde(default)]
    pub stream: bool,
    /// Maximum number of results
    #[serde(default)]
    pub limit: usize,
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

/// Mount request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountRequest {
    /// Filesystem type (plugin name)
    #[serde(rename = "fstype")]
    pub fstype: String,
    /// Mount path
    pub path: String,
    /// Plugin configuration
    #[serde(default)]
    pub config: serde_json::Value,
}

/// Unmount request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmountRequest {
    /// Mount path to unmount
    pub path: String,
}

/// Mount info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountInfo {
    /// Mount path
    pub path: String,
    /// Plugin name
    #[serde(rename = "pluginName")]
    pub plugin_name: String,
    /// Plugin configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

/// List mounts response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMountsResponse {
    /// List of mounted plugins
    pub mounts: Vec<MountInfo>,
}

/// Plugin info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin name
    pub name: String,
    /// Plugin mount path (if mounted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Whether this is an external plugin
    #[serde(rename = "isExternal")]
    pub is_external: bool,
    /// Configuration parameters
    #[serde(rename = "configParams")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_params: Option<Vec<agfs_sdk::ConfigParameter>>,
}

/// List plugins response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPluginsResponse {
    /// List of all plugins
    pub plugins: Vec<PluginInfo>,
}

/// Load plugin request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadPluginRequest {
    /// Path to the plugin library
    #[serde(rename = "library_path")]
    pub library_path: String,
}

/// Load plugin response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadPluginResponse {
    /// Success message
    pub message: String,
    /// Plugin name
    #[serde(rename = "pluginName")]
    pub plugin_name: String,
    /// Original plugin name (if renamed)
    #[serde(rename = "originalName")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_name: Option<String>,
    /// Whether the plugin was renamed
    pub renamed: bool,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Server status
    pub status: String,
    /// Server version
    pub version: String,
    /// Git commit hash
    #[serde(rename = "gitCommit")]
    pub git_commit: String,
    /// Build time
    #[serde(rename = "buildTime")]
    pub build_time: String,
}

/// Capabilities response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesResponse {
    /// Server version
    pub version: String,
    /// Supported features
    pub features: Vec<String>,
}

/// Handle open request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleOpenRequest {
    /// File path
    pub path: String,
    /// Open flags
    pub flags: Option<i32>,
    /// File mode for creation
    pub mode: Option<u32>,
    /// Read-only flag
    pub readonly: Option<bool>,
}

/// Handle open response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleOpenResponse {
    /// Assigned handle ID
    #[serde(rename = "handleId")]
    pub handle_id: i64,
    /// File path
    pub path: String,
    /// Open flags
    pub flags: i32,
    /// Lease duration in seconds
    pub lease: i32,
    /// When the lease expires
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}

/// Handle info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleInfoResponse {
    /// Handle ID
    #[serde(rename = "handleId")]
    pub handle_id: i64,
    /// File path
    pub path: String,
    /// Open flags
    #[serde(default)]
    pub flags: i32,
    /// Lease duration in seconds
    #[serde(default)]
    pub lease: i32,
    /// When the lease expires
    #[serde(rename = "expiresAt", default)]
    pub expires_at: String,
    /// When the handle was created
    #[serde(rename = "createdAt", default)]
    pub created_at: String,
    /// Last access time
    #[serde(rename = "lastAccess", default)]
    pub last_access: String,
    /// Read-only flag
    #[serde(default)]
    pub readonly: bool,
}

/// Handle read response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleReadResponse {
    /// Number of bytes read
    #[serde(rename = "bytesRead")]
    pub bytes_read: i32,
    /// Current position after read
    pub position: i64,
}

/// Handle write response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleWriteResponse {
    /// Number of bytes written
    #[serde(rename = "bytesWritten")]
    pub bytes_written: i32,
    /// Current position after write
    pub position: i64,
}

/// Handle seek response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleSeekResponse {
    /// New position
    pub position: i64,
}

/// Handle list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleListResponse {
    /// List of active handles
    pub handles: Vec<HandleInfoResponse>,
    /// Total count
    pub count: i32,
    /// Maximum handles
    pub max: i32,
}

/// Handle renew response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleRenewResponse {
    /// New lease expiration
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
    /// Lease duration in seconds
    pub lease: i32,
}

/// Handle close response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleCloseResponse {
    /// Handle ID that was closed
    #[serde(rename = "handleId")]
    pub handle_id: i64,
    /// Message
    pub message: String,
}

/// Handle read request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleReadRequest {
    /// Offset to read from (-1 for current position)
    pub offset: Option<i64>,
    /// Number of bytes to read
    pub size: Option<i64>,
}

/// Handle write request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleWriteRequest {
    /// Offset to write to (-1 for append)
    pub offset: Option<i64>,
    /// Base64 encoded data to write
    pub data: String,
    /// Whether to flush after write
    pub flush: Option<bool>,
}
