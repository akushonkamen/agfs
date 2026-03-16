//! Plugin management handlers

use super::response::{
    ListMountsResponse, ListPluginsResponse, LoadPluginRequest,
    MountInfo, MountRequest, PluginInfo, UnmountRequest,
};
use super::{error_response, map_error_to_status, success_response};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;
use std::collections::HashMap;

use crate::handlers::files::HandlerState;

/// GET /api/v1/mounts - List all mounted plugins
pub async fn list_mounts(
    State(state): State<HandlerState>,
) -> Result<Response, Response> {
    let mounts = state.mfs.get_mounts();

    let mount_infos: Vec<MountInfo> = mounts
        .iter()
        .map(|m| MountInfo {
            path: m.path.clone(),
            plugin_name: m.plugin.name().to_string(),
            config: None, // TODO: include config if needed
        })
        .collect();

    let response = ListMountsResponse { mounts: mount_infos };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/mount - Mount a plugin
pub async fn mount(
    State(state): State<HandlerState>,
    Json(req): Json<MountRequest>,
) -> Result<Response, Response> {
    if req.fstype.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "fstype is required"));
    }

    if req.path.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "path is required"));
    }

    // Convert config to HashMap
    let config: HashMap<String, Value> = if req.config.is_null() {
        HashMap::new()
    } else {
        // Try to parse as object
        match req.config {
            Value::Object(map) => map.into_iter().collect(),
            _ => return Err(error_response(StatusCode::BAD_REQUEST, "config must be an object")),
        }
    };

    state
        .mfs
        .mount_plugin(&req.fstype, &req.path, config)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response("plugin mounted"))
}

/// POST /api/v1/unmount - Unmount a plugin
pub async fn unmount(
    State(state): State<HandlerState>,
    Json(req): Json<UnmountRequest>,
) -> Result<Response, Response> {
    if req.path.is_empty() {
        return Err(error_response(StatusCode::BAD_REQUEST, "path is required"));
    }

    state
        .mfs
        .unmount(&req.path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response("plugin unmounted"))
}

/// GET /api/v1/plugins - List all plugins
pub async fn list_plugins(
    State(state): State<HandlerState>,
) -> Result<Response, Response> {
    let mounts = state.mfs.get_mounts();
    let builtin_names = state.mfs.get_builtin_plugin_names();

    // Build plugin info from mounts and builtin plugins
    let mut plugins = std::collections::HashMap::new();

    // Add mounted plugins
    for m in mounts {
        let name = m.plugin.name().to_string();
        plugins.entry(name.clone()).or_insert_with(|| PluginInfo {
            name: name.clone(),
            path: Some(m.path.clone()),
            is_external: false,
            config_params: Some(m.plugin.get_config_params()),
        });
    }

    // Add unmounted builtin plugins
    for name in builtin_names {
        plugins.entry(name.clone()).or_insert_with(|| {
            // Try to create a temp instance to get config params
            let config_params = state
                .mfs
                .create_plugin(&name)
                .map(|p| p.get_config_params())
                .unwrap_or_default();

            PluginInfo {
                name,
                path: None,
                is_external: false,
                config_params: Some(config_params),
            }
        });
    }

    let response = ListPluginsResponse {
        plugins: plugins.into_values().collect(),
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/v1/plugins/load - Load an external plugin
pub async fn load_plugin(
    State(_state): State<HandlerState>,
    Json(_req): Json<LoadPluginRequest>,
) -> Result<Response, Response> {
    // TODO: Implement external plugin loading
    Err(error_response(
        StatusCode::NOT_IMPLEMENTED,
        "external plugin loading not yet implemented",
    ))
}

/// POST /api/v1/plugins/unload - Unload an external plugin
pub async fn unload_plugin(
    State(_state): State<HandlerState>,
) -> Result<Response, Response> {
    // TODO: Implement external plugin unloading
    Err(error_response(
        StatusCode::NOT_IMPLEMENTED,
        "external plugin unloading not yet implemented",
    ))
}
