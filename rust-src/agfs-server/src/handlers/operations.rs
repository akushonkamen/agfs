//! Operation handlers for stat, rename, chmod, touch, truncate, symlink, readlink

use super::response::{
    ChmodRequest, DigestRequest, DigestResponse, ReadlinkResponse,
    RenameRequest, SymlinkRequest,
};
use super::{error_response, file_info_response, map_error_to_status, success_response};
use agfs_sdk::{AgfsError, FileSystem, Symlinker, Toucher, Truncater};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use md5 as md5_pkg;
use std::hash::Hasher;
use xxhash_rust::xxh3::Xxh3;

use crate::handlers::files::HandlerState;
use crate::mountablefs::MountableFS;

/// Query parameters for operations
#[derive(Debug, serde::Deserialize)]
pub struct OperationQuery {
    /// Path for the operation
    pub path: Option<String>,
    /// Size for truncate operation
    pub size: Option<i64>,
}

/// GET /api/v1/stat - Get file information
pub async fn stat(
    State(state): State<HandlerState>,
    Query(query): Query<OperationQuery>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    let info = state
        .mfs
        .stat(&path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok((StatusCode::OK, Json(file_info_response(info))).into_response())
}

/// POST /api/v1/rename - Rename/move a file or directory
pub async fn rename(
    State(state): State<HandlerState>,
    Query(query): Query<OperationQuery>,
    Json(req): Json<RenameRequest>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    if req.new_path.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "newPath is required"));
    }

    state
        .mfs
        .rename(&path, &req.new_path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response("renamed"))
}

/// POST /api/v1/chmod - Change file permissions
pub async fn chmod(
    State(state): State<HandlerState>,
    Query(query): Query<OperationQuery>,
    Json(req): Json<ChmodRequest>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    state
        .mfs
        .chmod(&path, req.mode)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response("permissions changed"))
}

/// POST /api/v1/touch - Update file modification time
pub async fn touch(
    State(state): State<HandlerState>,
    Query(query): Query<OperationQuery>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    // Try using Toucher trait if available
    match state.mfs.touch(&path) {
        Ok(_) => Ok(success_response("touched")),
        Err(_) => {
            // Fallback to read+write
            let info = state.mfs.stat(&path);
            if let Ok(info) = info {
                if !info.is_dir {
                    let data = state.mfs.read(&path, 0, -1).map_err(|e| {
                        error_response(map_error_to_status(&e), e.to_string())
                    })?;
                    state
                        .mfs
                        .write(&path, &data, -1, agfs_sdk::WriteFlag::TRUNCATE)
                        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;
                    return Ok(success_response("touched"));
                }
                return Err(error_response(
                    StatusCode::BAD_REQUEST,
                    "cannot touch directory",
                ));
            }
            // File doesn't exist, create it
            state
                .mfs
                .write(&path, &[], -1, agfs_sdk::WriteFlag::CREATE)
                .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;
            Ok(success_response("touched"))
        }
    }
}

/// POST /api/v1/truncate - Truncate a file
pub async fn truncate(
    State(state): State<HandlerState>,
    Query(query): Query<OperationQuery>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    let size = query.size.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "size parameter is required")
    })?;

    if size < 0 {
        return Err(error_response(StatusCode::BAD_REQUEST, "size must be non-negative"));
    }

    state
        .mfs
        .truncate(&path, size)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response("truncated"))
}

/// POST /api/v1/symlink - Create a symbolic link
pub async fn symlink(
    State(state): State<HandlerState>,
    Query(query): Query<OperationQuery>,
    Json(req): Json<SymlinkRequest>,
) -> Result<Response, Response> {
    let link_path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required (link path)")
    })?;

    if req.target.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "target is required"));
    }

    state
        .mfs
        .symlink(&req.target, &link_path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response("symlink created"))
}

/// GET /api/v1/readlink - Read symlink target
pub async fn readlink(
    State(state): State<HandlerState>,
    Query(query): Query<OperationQuery>,
) -> Result<Response, Response> {
    let link_path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    let target = state
        .mfs
        .readlink(&link_path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok((StatusCode::OK, Json(ReadlinkResponse { target })).into_response())
}

/// POST /api/v1/digest - Calculate file digest (hash)
pub async fn digest(
    State(state): State<HandlerState>,
    Json(req): Json<DigestRequest>,
) -> Result<Response, Response> {
    if req.algorithm != "xxh3" && req.algorithm != "md5" {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            format!("unsupported algorithm: {} (supported: xxh3, md5)", req.algorithm),
        ));
    }

    if req.path.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "path is required"));
    }

    let digest = match req.algorithm.as_str() {
        "xxh3" => calculate_xxh3(&state.mfs, &req.path)
            .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?,
        "md5" => calculate_md5(&state.mfs, &req.path)
            .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?,
        _ => unreachable!(),
    };

    let response = DigestResponse {
        algorithm: req.algorithm,
        path: req.path,
        digest,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Calculate XXH3 hash for a file
fn calculate_xxh3(mfs: &MountableFS, path: &str) -> Result<String, AgfsError> {
    // Open file for streaming read
    let mut reader = mfs.open(path)?;

    // Stream and hash in chunks
    let mut hasher = Xxh3::new();
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

    loop {
        use std::io::Read;
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.write(&buffer[..n]);
    }

    let hash = hasher.finish();
    Ok(format!("{:016x}", hash))
}

/// Calculate MD5 hash for a file
fn calculate_md5(mfs: &MountableFS, path: &str) -> Result<String, AgfsError> {
    // Open file for streaming read
    let mut reader = mfs.open(path)?;

    // Stream and hash in chunks
    let mut hasher = md5_pkg::Context::new();
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

    loop {
        use std::io::Read;
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.consume(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.compute()))
}
