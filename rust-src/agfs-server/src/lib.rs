//! AGFS Server - Rust implementation
//!
//! This crate implements the AGFS file system server with plugin support.

#![warn(missing_docs)]
// TODO: Remove this allow once full implementation is complete
#![allow(missing_docs)]

pub mod config;
pub mod filesystem;
pub mod handlers;
pub mod mountablefs;
pub mod plugin;
pub mod plugins;

pub use config::Config;
pub use mountablefs::MountableFS;
