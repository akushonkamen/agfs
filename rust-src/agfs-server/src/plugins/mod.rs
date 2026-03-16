//! Plugin implementations for AGFS server
//!
//! This module contains all plugin implementations.

pub mod devfs;
pub mod empty;
pub mod gptfs;
pub mod hellofs;
pub mod httpfs;
pub mod kvfs;
pub mod localfs;
pub mod memfs;
pub mod proxyfs;
pub mod queuefs;
pub mod s3fs;
// pub mod sqlfs;  // TODO: fix compilation errors
pub mod sqlfs2;
pub mod streamfs;
pub mod streamrotatefs;
pub mod vectorfs;

// Re-export plugin factory functions
pub use empty::create_empty_plugin;
pub use localfs::create_localfs_plugin;
pub use memfs::create_memfs_plugin;
pub use queuefs::create_queuefs_plugin;
