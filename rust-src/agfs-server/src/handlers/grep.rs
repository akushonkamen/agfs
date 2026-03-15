//! Grep handler for file content search

use super::response::{GrepMatch, GrepResponse, GrepRequest};
use super::{error_response, map_error_to_status};
use agfs_sdk::{AgfsError, FileSystem};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use regex::Regex;

use crate::handlers::files::HandlerState;
use crate::mountablefs::MountableFS;
/// POST /api/v1/grep - Search for pattern in files
pub async fn grep(
    State(state): State<HandlerState>,
    Json(req): Json<GrepRequest>,
) -> Result<Response, Response> {
    if req.path.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "path is required"));
    }

    if req.pattern.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "pattern is required"));
    }

    // Compile regex pattern
    let re = Regex::new(&req.pattern).map_err(|_| {
        error_response(StatusCode::BAD_REQUEST, format!("invalid regex pattern: {}", req.pattern))
    })?;

    // Check if path exists
    let info = state
        .mfs
        .stat(&req.path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    let matches = if info.is_dir {
        if !req.recursive {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "path is a directory, use recursive=true to search",
            ));
        }
        grep_directory_recursive(&state.mfs, &req.path, &re)
            .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?
    } else {
        grep_file(&state.mfs, &req.path, &re)
            .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?
    };

    let response = GrepResponse {
        count: matches.len() as i32,
        matches,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Search in a single file
fn grep_file(mfs: &MountableFS, path: &str, re: &Regex) -> Result<Vec<GrepMatch>, AgfsError> {
    let data = mfs.read(path, 0, -1)?;
    let content = String::from_utf8_lossy(&data);

    let mut matches = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        if re.is_match(line) {
            matches.push(GrepMatch {
                file: path.to_string(),
                line: (line_num + 1) as i32,
                content: line.to_string(),
            });
        }
    }

    Ok(matches)
}

/// Recursively search in a directory
fn grep_directory_recursive(
    mfs: &MountableFS,
    dir_path: &str,
    re: &Regex,
) -> Result<Vec<GrepMatch>, AgfsError> {
    let mut all_matches = Vec::new();

    let entries = mfs.read_dir(dir_path)?;
    for entry in entries {
        let full_path = if dir_path.ends_with('/') {
            format!("{}{}", dir_path, entry.name)
        } else {
            format!("{}/{}", dir_path, entry.name)
        };

        if entry.is_dir {
            // Recursively search subdirectory
            match grep_directory_recursive(mfs, &full_path, re) {
                Ok(mut matches) => all_matches.append(&mut matches),
                Err(_) => {
                    // Log error but continue searching other files
                    continue;
                }
            }
        } else {
            // Search in file
            match grep_file(mfs, &full_path, re) {
                Ok(mut matches) => all_matches.append(&mut matches),
                Err(_) => {
                    // Log error but continue searching other files
                    continue;
                }
            }
        }
    }

    Ok(all_matches)
}
