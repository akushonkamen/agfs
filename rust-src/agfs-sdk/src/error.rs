//! AGFS error types

// TODO: Remove this allow once full implementation is complete
#![allow(missing_docs)]

use thiserror::Error;

/// AGFS unified error type
#[derive(Error, Debug)]
pub enum AgfsError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("other error: {0}")]
    Other(String),
}
