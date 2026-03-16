//! Plugin trait and related types
//!
//! This module defines the plugin interface that all file system plugins must implement.
//! Based on the Go implementation in `agfs-server/pkg/plugin/plugin.go`.

use crate::error::AgfsError;
use crate::filesystem::FileSystem;
use crate::types::ConfigParameter;
use serde_json::Value;
use std::collections::HashMap;

/// Service plugin trait
///
/// This trait defines the interface that all service plugins must implement.
/// Each plugin acts as a virtual file system providing service-specific operations.
///
/// # Thread Safety
/// All plugin implementations must be thread-safe (`Send + Sync`).
///
/// # Lifecycle
/// 1. Plugin is created with default configuration
/// 2. `validate()` is called to check configuration
/// 3. `initialize()` is called to set up the plugin
/// 4. Plugin can now handle file operations via `get_filesystem()`
/// 5. `shutdown()` is called when the plugin is being unloaded
///
/// Corresponds to Go `ServicePlugin` interface in `agfs-server/pkg/plugin/plugin.go`.
pub trait ServicePlugin: Send + Sync {
    /// Get the plugin name
    ///
    /// Returns a unique identifier for this plugin type.
    fn name(&self) -> &str;

    /// Validate the plugin configuration
    ///
    /// This method is called before initialization to check that the configuration
    /// is valid. It should verify all required parameters are present and have valid values.
    ///
    /// # Arguments
    /// - `config`: Configuration map with parameter names as keys
    ///
    /// # Returns
    /// `Ok(())` if configuration is valid, `Err(AgfsError)` if invalid.
    fn validate(&self, config: &HashMap<String, Value>) -> Result<(), AgfsError>;

    /// Initialize the plugin with configuration
    ///
    /// This method is called after validation succeeds. It should set up any
    /// internal state required for the plugin to function.
    ///
    /// # Arguments
    /// - `config`: Configuration map with parameter names as keys
    ///
    /// # Returns
    /// `Ok(())` if initialization succeeded, `Err(AgfsError)` if it failed.
    fn initialize(&mut self, config: HashMap<String, Value>) -> Result<(), AgfsError>;

    /// Get the filesystem implementation
    ///
    /// Returns a reference to the FileSystem implementation for this plugin.
    /// This allows the plugin to handle file operations in a service-specific way.
    fn get_filesystem(&self) -> &dyn FileSystem;

    /// Get the plugin README content
    ///
    /// Returns documentation about the plugin's functionality and usage.
    /// This should be in markdown format.
    fn get_readme(&self) -> &str;

    /// Get the configuration parameters
    ///
    /// Returns metadata about what configuration options this plugin supports.
    /// This is used for auto-generating help text and validation.
    fn get_config_params(&self) -> Vec<ConfigParameter>;

    /// Shutdown the plugin
    ///
    /// Called when the plugin is being unloaded. Should release any resources
    /// and stop any background operations.
    fn shutdown(&mut self) -> Result<(), AgfsError>;
}

/// Mount point representation
///
/// Represents a mounted service plugin at a specific path.
/// Corresponds to Go `MountPoint` in `agfs-server/pkg/plugin/plugin.go`.
pub struct MountPoint {
    /// The path where the plugin is mounted
    pub path: String,

    /// The mounted plugin instance
    pub plugin: Box<dyn ServicePlugin>,
}

impl std::fmt::Debug for MountPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MountPoint")
            .field("path", &self.path)
            .field("plugin", &self.plugin.name())
            .finish()
    }
}

/// Plugin metadata
///
/// Contains information about a plugin type.
/// Corresponds to Go `PluginMetadata` in `agfs-server/pkg/plugin/plugin.go`.
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Plugin description
    pub description: String,

    /// Plugin author
    pub author: String,
}
