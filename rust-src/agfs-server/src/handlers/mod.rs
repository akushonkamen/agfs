//! HTTP handlers for AGFS server
//!
//! This module contains all HTTP request handlers for the AGFS API.
//! Based on Go implementation in `agfs-server/pkg/handlers/handlers.go`.

pub mod directories;
pub mod files;
pub mod grep;
pub mod operations;
pub mod plugins;
pub mod response;
pub mod system;

// Re-export common types
pub use response::{ErrorResponse, SuccessResponse};

use agfs_sdk::AgfsError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Map AgfsError to HTTP status code
pub fn map_error_to_status(err: &AgfsError) -> StatusCode {
    match err {
        AgfsError::NotFound(_) => StatusCode::NOT_FOUND,
        AgfsError::PermissionDenied(_) => StatusCode::FORBIDDEN,
        AgfsError::InvalidArgument(_) => StatusCode::BAD_REQUEST,
        AgfsError::AlreadyExists(_) => StatusCode::CONFLICT,
        AgfsError::NotSupported => StatusCode::NOT_IMPLEMENTED,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Create a JSON error response
pub fn error_response(status: StatusCode, message: impl Into<String>) -> Response {
    let body = json!({
        "error": message.into()
    });
    (status, Json(body)).into_response()
}

/// Create a JSON success response
pub fn success_response(message: impl Into<String>) -> Response {
    let body = json!({
        "message": message.into()
    });
    (StatusCode::OK, Json(body)).into_response()
}

/// Convert FileInfo to API response format
pub fn file_info_response(info: agfs_sdk::FileInfo) -> serde_json::Value {
    json!({
        "name": info.name,
        "size": info.size,
        "mode": info.mode,
        "modTime": info.mod_time.to_rfc3339(),
        "isDir": info.is_dir,
        "meta": info.meta,
    })
}

/// Convert list of FileInfo to API response format
pub fn list_response(infos: Vec<agfs_sdk::FileInfo>) -> serde_json::Value {
    let files: Vec<serde_json::Value> = infos.into_iter().map(file_info_response).collect();
    json!({ "files": files })
}
