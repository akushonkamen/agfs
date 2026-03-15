//! Plugin implementations for AGFS server
//!
//! This module contains all plugin implementations.

pub mod empty;

// Re-export plugin factory functions
pub use empty::create_empty_plugin;
