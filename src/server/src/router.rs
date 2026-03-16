//! Main router for AGFS server
//!
//! Sets up all axum routes and middleware.

use crate::handlers::{directories, files, grep, handles, operations, plugins, system};
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::mountablefs::MountableFS;

/// Create the main axum router
pub fn create_router(mfs: Arc<MountableFS>) -> Router {
    let state = files::HandlerState { mfs };

    Router::new()
        // Root and system endpoints
        .route("/", get(system::root))
        .route("/api/v1/health", get(system::health))
        .route("/api/v1/capabilities", get(system::capabilities))
        .route("/api/v1/version", get(system::version))
        // File operations
        .route("/api/v1/files", get(files::read_file).post(files::create_file).put(files::write_file))
        .route("/api/v1/files/delete", post(files::delete_file))
        // Directory operations
        .route("/api/v1/directories", get(directories::list_directory).post(directories::create_directory))
        .route("/api/v1/directories/delete", post(directories::delete_directory))
        // Operations
        .route("/api/v1/stat", get(operations::stat))
        .route("/api/v1/rename", post(operations::rename))
        .route("/api/v1/chmod", post(operations::chmod))
        .route("/api/v1/touch", post(operations::touch))
        .route("/api/v1/truncate", post(operations::truncate))
        .route("/api/v1/symlink", post(operations::symlink))
        .route("/api/v1/readlink", get(operations::readlink))
        .route("/api/v1/digest", post(operations::digest))
        .route("/api/v1/grep", post(grep::grep))
        // Handle operations
        .route("/api/v1/handles/open", post(handles::open_handle))
        .route("/api/v1/handles/:id/read", post(handles::read_handle))
        .route("/api/v1/handles/:id/write", post(handles::write_handle))
        .route("/api/v1/handles/:id/close", post(handles::close_handle))
        .route("/api/v1/handles/:id", get(handles::get_handle_info))
        .route("/api/v1/handles/:id/delete", post(handles::delete_handle))
        // Plugin management
        .route("/api/v1/mounts", get(plugins::list_mounts))
        .route("/api/v1/mount", post(plugins::mount))
        .route("/api/v1/unmount", post(plugins::unmount))
        .route("/api/v1/plugins", get(plugins::list_plugins))
        .route("/api/v1/plugins/load", post(plugins::load_plugin))
        .route("/api/v1/plugins/unload", post(plugins::unload_plugin))
        .with_state(state)
}
