//! AGFS Server - Rust implementation
//!
//! This crate implements the AGFS file system server with plugin support.

#![warn(missing_docs)]
// TODO: Remove this allow once full implementation is complete
#![allow(missing_docs)]

pub mod config;
pub mod handlers;
pub mod mountablefs;
pub mod plugins;
pub mod router;
pub mod traffic_monitor;

pub use config::Config;
pub use mountablefs::{MountableFS, PluginFactory, META_VALUE_ROOT, META_VALUE_MOUNT_POINT};
pub use traffic_monitor::{SharedTrafficMonitor, TrafficMonitor, TrafficStats};
