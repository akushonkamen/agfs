//! File operation handlers
//!
//! Handles GET/POST/PUT/DELETE requests for /api/v1/files endpoint.

use super::{error_response, map_error_to_status, success_response};
use agfs_sdk::{FileSystem, WriteFlag};
use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::mountablefs::MountableFS;

/// Handler state shared across all handlers
#[derive(Clone)]
pub struct HandlerState {
    pub mfs: Arc<MountableFS>,
}

/// Query parameters for file operations
#[derive(Debug, Deserialize)]
pub struct FileQuery {
    /// File path
    pub path: Option<String>,
    /// Read offset (for GET)
    pub offset: Option<i64>,
    /// Read size (for GET)
    pub size: Option<i64>,
    /// Enable streaming (for GET)
    pub stream: Option<bool>,
    /// Recursive delete (for DELETE)
    pub recursive: Option<bool>,
}

/// POST /api/v1/files - Create a new file
pub async fn create_file(
    State(state): State<HandlerState>,
    Query(query): Query<FileQuery>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    state
        .mfs
        .create(&path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response("file created"))
}

/// GET /api/v1/files - Read file content
pub async fn read_file(
    State(state): State<HandlerState>,
    Query(query): Query<FileQuery>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    // Check if streaming is requested
    if query.stream.unwrap_or(false) {
        // TODO: Implement streaming
        return Err(error_response(
            StatusCode::NOT_IMPLEMENTED,
            "streaming not yet implemented",
        ));
    }

    let offset = query.offset.unwrap_or(0);
    let size = query.size.unwrap_or(-1);

    let data = state
        .mfs
        .read(&path, offset, size)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        data,
    )
        .into_response())
}

/// PUT /api/v1/files - Write file content
pub async fn write_file(
    State(state): State<HandlerState>,
    Query(query): Query<FileQuery>,
    body: Body,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    // Read request body
    let bytes = axum::body::to_bytes(body, 10 * 1024 * 1024) // 10MB limit
        .await
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, "failed to read request body"))?;

    let flags = WriteFlag::CREATE | WriteFlag::TRUNCATE;
    let written = state
        .mfs
        .write(&path, &bytes, -1, flags)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response(format!("Written {} bytes", written)))
}

/// DELETE /api/v1/files - Delete file or directory
pub async fn delete_file(
    State(state): State<HandlerState>,
    Query(query): Query<FileQuery>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    let recursive = query.recursive.unwrap_or(false);

    if recursive {
        state
            .mfs
            .remove_all(&path)
            .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;
    } else {
        state
            .mfs
            .remove(&path)
            .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;
    }

    Ok(success_response("deleted"))
}
