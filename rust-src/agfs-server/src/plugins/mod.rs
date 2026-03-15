//! Plugin implementations for AGFS server
//!
//! This module contains all plugin implementations.

pub mod devfs;
pub mod empty;
pub mod hellofs;
pub mod kvfs;
pub mod memfs;
pub mod queuefs;
pub mod streamfs;
pub mod streamrotatefs;

// Re-export plugin factory functions
pub use empty::create_empty_plugin;
