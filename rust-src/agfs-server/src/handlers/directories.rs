//! Directory operation handlers
//!
//! Handles GET/POST/DELETE requests for /api/v1/directories endpoint.

use super::response::ListResponse;
use super::{error_response, map_error_to_status, success_response};
use agfs_sdk::FileSystem;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use crate::handlers::files::HandlerState;

/// Query parameters for directory operations
#[derive(Debug, Deserialize)]
pub struct DirectoryQuery {
    /// Directory path
    pub path: Option<String>,
    /// Directory permissions (for POST, octal)
    pub mode: Option<String>,
}

/// GET /api/v1/directories - List directory contents
pub async fn list_directory(
    State(state): State<HandlerState>,
    Query(query): Query<DirectoryQuery>,
) -> Result<Response, Response> {
    let path = query.path.unwrap_or_else(|| "/".to_string());

    let infos = state
        .mfs
        .read_dir(&path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    let response = ListResponse {
        files: infos
            .into_iter()
            .map(|info| crate::handlers::response::FileInfoResponse {
                name: info.name,
                size: info.size,
                mode: info.mode,
                mod_time: info.mod_time.to_rfc3339(),
                is_dir: info.is_dir,
                meta: if info.meta.content.is_empty() {
                    None
                } else {
                    Some(serde_json::to_value(info.meta).unwrap_or_default())
                },
            })
            .collect(),
    };

    Ok((StatusCode::OK, axum::Json(response)).into_response())
}

/// POST /api/v1/directories - Create a directory
pub async fn create_directory(
    State(state): State<HandlerState>,
    Query(query): Query<DirectoryQuery>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    // Parse mode (default to 0755)
    let mode = query
        .mode
        .and_then(|m| u32::from_str_radix(&m, 8).ok())
        .unwrap_or(0o755);

    state
        .mfs
        .mkdir(&path, mode)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response("directory created"))
}

/// DELETE /api/v1/directories - Delete a directory
pub async fn delete_directory(
    State(state): State<HandlerState>,
    Query(query): Query<DirectoryQuery>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    state
        .mfs
        .remove_all(&path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response("deleted"))
}
