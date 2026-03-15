//! Plugin implementations for AGFS server
//!
//! This module contains all plugin implementations.

pub mod devfs;
pub mod empty;
pub mod hellofs;
pub mod kvfs;
pub mod localfs;
pub mod memfs;
pub mod queuefs;
pub mod s3fs;
pub mod sqlfs;
pub mod sqlfs2;
pub mod streamfs;
pub mod streamrotatefs;

// Re-export plugin factory functions
pub use empty::create_empty_plugin;
pub use localfs::create_localfs_plugin;
