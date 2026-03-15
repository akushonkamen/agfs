//! AGFS SDK - Rust implementation
//!
//! This crate provides the core types, traits, and HTTP client for interacting with AGFS server.
//!
//! # Modules
//!
//! - [`client`] - HTTP client for AGFS server API
//! - [`error`] - Unified error types
//! - [`types`] - Common data structures
//! - [`filesystem`] - FileSystem trait and extension traits
//! - [`plugin`] - ServicePlugin trait and plugin types

#![warn(missing_docs)]

pub mod client;
pub mod error;
pub mod filesystem;
pub mod plugin;
pub mod types;

// Re-export commonly used items
pub use client::Client;
pub use error::AgfsError;
pub use filesystem::{FileSystem, StreamReader, Streamer, Symlinker, Toucher, Truncater};
pub use plugin::{MountPoint, PluginMetadata, ServicePlugin};
pub use types::{
    ConfigParameter, CapabilitiesResponse, DigestRequest, DigestResponse, ErrorResponse,
    FileInfo, GrepMatch, GrepRequest, GrepResponse, HandleInfo, HandleResponse, ListResponse,
    MetaData, OpenFlag, ReadlinkResponse, RenameRequest, SymlinkRequest, SuccessResponse,
    WriteFlag,
};
