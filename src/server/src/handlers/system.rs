//! System endpoints - health, capabilities, and root

use super::response::{CapabilitiesResponse, HealthResponse};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::handlers::files::HandlerState;
/// Server version info
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_COMMIT: &str = "dev";
pub const BUILD_TIME: &str = "unknown";

/// GET / - Root endpoint
pub async fn root() -> Response {
    let body = serde_json::json!({
        "service": "AGFS Server",
        "version": VERSION,
        "status": "running"
    });
    (StatusCode::OK, Json(body)).into_response()
}

/// GET /api/v1/health - Health check
pub async fn health(State(_state): State<HandlerState>) -> Response {
    // TODO: Check actual health status
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: VERSION.to_string(),
        git_commit: GIT_COMMIT.to_string(),
        build_time: BUILD_TIME.to_string(),
    };
    (StatusCode::OK, Json(response)).into_response()
}

/// GET /api/v1/capabilities - Server capabilities
pub async fn capabilities() -> Response {
    let response = CapabilitiesResponse {
        version: VERSION.to_string(),
        features: vec![
            "handlefs".to_string(),
            "grep".to_string(),
            "digest".to_string(),
            "stream".to_string(),
            "touch".to_string(),
            "truncate".to_string(),
            "symlink".to_string(),
        ],
    };
    (StatusCode::OK, Json(response)).into_response()
}

/// GET /api/v1/version - Version info
pub async fn version() -> Response {
    let body = serde_json::json!({
        "version": VERSION,
        "gitCommit": GIT_COMMIT,
        "buildTime": BUILD_TIME
    });
    (StatusCode::OK, Json(body)).into_response()
}
