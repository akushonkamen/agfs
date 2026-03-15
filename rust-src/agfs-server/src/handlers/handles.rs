//! File handle API handlers
//!
//! Handles /api/v1/handles/* endpoints for stateful file operations.
//! Based on the Go implementation in `agfs-server/pkg/handlers/handles.go`.

use super::response::{
    HandleCloseResponse, HandleInfoResponse, HandleOpenRequest, HandleReadRequest,
    HandleWriteRequest,
};
use super::{error_response, map_error_to_status, success_response};
use agfs_sdk::WriteFlag;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use crate::handlers::files::HandlerState;

/// POST /api/v1/handles/open - Open a file and get a handle ID
pub async fn open_handle(
    State(state): State<HandlerState>,
    Json(req): Json<HandleOpenRequest>,
) -> Result<Response, Response> {
    if req.path.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "path is required"));
    }

    // Parse flags from request
    let flags = if req.readonly.unwrap_or(false) {
        WriteFlag::NONE
    } else {
        WriteFlag::CREATE | WriteFlag::TRUNCATE
    };

    // Get the mount point for this path
    let (mount_point, relative_path) = state
        .mfs
        .find_mount_and_relative_path(&req.path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    // Open the file
    let handle_id = state
        .mfs
        .allocate_handle(mount_point, relative_path, flags)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    let response = HandleInfoResponse {
        handle_id,
        path: req.path.clone(),
        flags: req.flags.unwrap_or(0),
        lease: 3600, // 1 hour default lease
        expires_at: format!("{}", chrono::Utc::now() + chrono::Duration::seconds(3600)),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_access: chrono::Utc::now().to_rfc3339(),
        readonly: req.readonly.unwrap_or(false),
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/handles/:id/read - Read from a handle
pub async fn read_handle(
    State(state): State<HandlerState>,
    Path(id): Path<String>,
    Json(req): Json<HandleReadRequest>,
) -> Result<Response, Response> {
    // Parse handle ID
    let handle_id = id
        .parse::<i64>()
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, "invalid handle ID"))?;

    // Get size to read
    let size = req.size.unwrap_or(4096);
    if size <= 0 || size > 10 * 1024 * 1024 {
        // Limit to 10MB per read
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "size must be between 1 and 10485760",
        ));
    }

    // Read from handle
    let data = state
        .mfs
        .read_handle(handle_id, req.offset.unwrap_or(0), size)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    // Return as base64 encoded data
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": BASE64.encode(&data),
            "size": data.len(),
        })),
    )
        .into_response())
}

/// POST /api/v1/handles/:id/write - Write to a handle
pub async fn write_handle(
    State(state): State<HandlerState>,
    Path(id): Path<String>,
    Json(req): Json<HandleWriteRequest>,
) -> Result<Response, Response> {
    // Parse handle ID
    let handle_id = id
        .parse::<i64>()
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, "invalid handle ID"))?;

    // Decode base64 data
    let data = BASE64
        .decode(&req.data)
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, "invalid base64 encoded data"))?;

    // Write to handle
    let written = state
        .mfs
        .write_handle(
            handle_id,
            req.offset.unwrap_or(-1),
            &data,
            req.flush.unwrap_or(false),
        )
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response(format!("Written {} bytes", written)))
}

/// POST /api/v1/handles/:id/close - Close a handle
pub async fn close_handle(
    State(state): State<HandlerState>,
    Path(id): Path<String>,
) -> Result<Response, Response> {
    // Parse handle ID
    let handle_id = id
        .parse::<i64>()
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, "invalid handle ID"))?;

    // Close the handle
    state
        .mfs
        .close_handle(handle_id)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    let response = HandleCloseResponse {
        handle_id,
        message: "handle closed".to_string(),
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// GET /api/v1/handles/:id - Get handle info
pub async fn get_handle_info(
    State(state): State<HandlerState>,
    Path(id): Path<String>,
) -> Result<Response, Response> {
    // Parse handle ID
    let handle_id = id
        .parse::<i64>()
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, "invalid handle ID"))?;

    // Get handle info
    let info = state
        .mfs
        .get_handle_info(handle_id)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    let response = HandleInfoResponse {
        handle_id,
        path: info.full_path,
        flags: 0,
        lease: 3600,
        expires_at: format!("{}", chrono::Utc::now() + chrono::Duration::seconds(3600)),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_access: chrono::Utc::now().to_rfc3339(),
        readonly: info.readonly,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// DELETE /api/v1/handles/:id - Delete/close a handle (alias for close)
pub async fn delete_handle(
    State(state): State<HandlerState>,
    Path(id): Path<String>,
) -> Result<Response, Response> {
    close_handle(State(state), Path(id)).await
}
