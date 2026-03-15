//! AGFS error types
//!
//! This module defines the unified error type used throughout AGFS.

use thiserror::Error;

/// AGFS unified error type
///
/// This enum represents all possible errors that can occur in AGFS operations.
/// It follows the error conventions from the Go implementation.
#[derive(Error, Debug)]
pub enum AgfsError {
    /// Resource not found (file, directory, or plugin)
    #[error("not found: {0}")]
    NotFound(String),

    /// Permission denied for the requested operation
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Resource already exists (file, directory, or mount point)
    #[error("already exists: {0}")]
    AlreadyExists(String),

    /// Operation not supported by this filesystem or plugin
    #[error("not supported")]
    NotSupported,

    /// Invalid argument provided
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// Path is not a directory when directory operation was expected
    #[error("not a directory: {0}")]
    NotDirectory(String),

    /// I/O error from the underlying system
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP client error
    #[error("http error: {0}")]
    Http(String),

    /// Serialization/deserialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Internal error (should not happen in normal operation)
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<reqwest::Error> for AgfsError {
    fn from(err: reqwest::Error) -> Self {
        AgfsError::Http(err.to_string())
    }
}

impl AgfsError {
    /// Create a not found error with context
    pub fn not_found(path: impl Into<String>) -> Self {
        AgfsError::NotFound(path.into())
    }

    /// Create a permission denied error with context
    pub fn permission_denied(path: impl Into<String>) -> Self {
        AgfsError::PermissionDenied(path.into())
    }

    /// Create an already exists error with context
    pub fn already_exists(path: impl Into<String>) -> Self {
        AgfsError::AlreadyExists(path.into())
    }

    /// Create an invalid argument error with context
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        AgfsError::InvalidArgument(msg.into())
    }

    /// Create an internal error with context
    pub fn internal(msg: impl Into<String>) -> Self {
        AgfsError::Internal(msg.into())
    }
}
