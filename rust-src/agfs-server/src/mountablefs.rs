//! Mountable file system - routes requests to mounted plugins
//!
//! This module implements the core routing layer that dispatches file system
//! operations to the appropriate mounted plugin based on path prefix matching.
//!
//! Based on Go implementation in `agfs-server/pkg/mountablefs/mountablefs.go`.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, ServicePlugin, StreamReader, Streamer, Symlinker, Toucher, Truncater, WriteFlag};
use chrono::Utc;
use dashmap::DashMap;
use radix_trie::{Trie, TrieCommon};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path as StdPath;
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, RwLock};

/// Meta value constants for MountableFS
pub const META_VALUE_ROOT: &str = "root";
pub const META_VALUE_MOUNT_POINT: &str = "mount-point";

/// Plugin factory function type
///
/// This function creates a new plugin instance.
pub type PluginFactory = fn() -> Box<dyn ServicePlugin>;

/// Mount point information
pub struct MountPoint {
    /// The path where this plugin is mounted
    pub path: String,
    /// The mounted plugin instance
    pub plugin: Box<dyn ServicePlugin>,
    /// The plugin configuration
    pub config: HashMap<String, Value>,
}

impl std::fmt::Debug for MountPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MountPoint")
            .field("path", &self.path)
            .field("plugin", &self.plugin.name())
            .finish()
    }
}

/// Information about a file handle
#[allow(dead_code)]
struct HandleInfo {
    /// The mount point where this handle was opened
    mount: Arc<MountPoint>,
    /// The local handle ID (plugin-specific)
    local_handle_id: i64,
    /// Full path including mount point
    full_path: String,
}

/// MountableFS - routes file system requests to mounted plugins
///
/// This is the core routing layer that uses a radix trie for efficient
/// longest-prefix matching of paths to mount points.
pub struct MountableFS {
    /// Radix trie for mount point routing (path -> MountPoint)
    /// Uses ArcSwap for lock-free reads
    mount_tree: Arc<RwLock<Trie<String, Arc<MountPoint>>>>,

    /// Registered plugin factories (name -> factory)
    plugin_factories: RwLock<HashMap<String, PluginFactory>>,

    /// Global handle ID counter (atomic, cross-plugin unique)
    #[allow(dead_code)]
    global_handle_id: AtomicI64,

    /// Handle information storage (global_handle_id -> HandleInfo)
    #[allow(dead_code)]
    handles: DashMap<i64, HandleInfo>,

    /// Symlink mapping table (link_path -> target_path)
    symlinks: DashMap<String, String>,
}

impl MountableFS {
    /// Create a new MountableFS
    pub fn new() -> Self {
        Self {
            mount_tree: Arc::new(RwLock::new(Trie::new())),
            plugin_factories: RwLock::new(HashMap::new()),
            global_handle_id: AtomicI64::new(0),
            handles: DashMap::new(),
            symlinks: DashMap::new(),
        }
    }

    /// Register a plugin factory for dynamic mounting
    pub fn register_plugin_factory(&self, name: &str, factory: PluginFactory) {
        let mut factories = self.plugin_factories.write().unwrap();
        factories.insert(name.to_string(), factory);
    }

    /// Create a plugin instance from a registered factory
    pub fn create_plugin(&self, name: &str) -> Option<Box<dyn ServicePlugin>> {
        let factories = self.plugin_factories.read().unwrap();
        let factory = factories.get(name)?;
        Some(factory())
    }

    /// Get list of all registered plugin names
    pub fn get_builtin_plugin_names(&self) -> Vec<String> {
        let factories = self.plugin_factories.read().unwrap();
        factories.keys().cloned().collect()
    }

    /// Mount a plugin at the specified path
    pub fn mount(&self, path: &str, plugin: Box<dyn ServicePlugin>, config: HashMap<String, Value>) -> Result<(), AgfsError> {
        let normalized_path = normalize_path(path);

        // Check if path is already mounted
        {
            let tree = self.mount_tree.read().unwrap();
            if tree.get(&normalized_path).is_some() {
                return Err(AgfsError::AlreadyExists(format!("mount point {}", normalized_path)));
            }
        }

        // Create mount point
        let mount_point = Arc::new(MountPoint {
            path: normalized_path.clone(),
            plugin,
            config,
        });

        // Insert into tree
        {
            let mut tree = self.mount_tree.write().unwrap();
            tree.insert(normalized_path.clone(), mount_point);
        }

        Ok(())
    }

    /// Mount a plugin by type name
    pub fn mount_plugin(
        &self,
        fstype: &str,
        path: &str,
        mut config: HashMap<String, Value>,
    ) -> Result<(), AgfsError> {
        let normalized_path = normalize_path(path);

        // Check if path is already mounted
        {
            let tree = self.mount_tree.read().unwrap();
            if tree.get(&normalized_path).is_some() {
                return Err(AgfsError::AlreadyExists(format!("mount point {}", normalized_path)));
            }
        }

        // Get plugin factory
        let factory = {
            let factories = self.plugin_factories.read().unwrap();
            *factories
                .get(fstype)
                .ok_or_else(|| AgfsError::InvalidArgument(format!("unknown filesystem type: {}", fstype)))?
        };

        // Create plugin instance
        let mut plugin = factory();

        // Inject mount_path into config
        config.insert("mount_path".to_string(), Value::String(normalized_path.clone()));

        // Validate plugin configuration
        plugin.validate(&config)
            .map_err(|e| AgfsError::Internal(format!("plugin validation failed: {}", e)))?;

        // Initialize plugin
        plugin
            .initialize(config)
            .map_err(|e| AgfsError::Internal(format!("plugin initialization failed: {}", e)))?;

        // Create mount point
        let mount_point = Arc::new(MountPoint {
            path: normalized_path.clone(),
            plugin,
            config: HashMap::new(), // Config already consumed by initialize
        });

        // Insert into tree
        {
            let mut tree = self.mount_tree.write().unwrap();
            tree.insert(normalized_path, mount_point);
        }

        Ok(())
    }

    /// Unmount a plugin from the specified path
    pub fn unmount(&self, path: &str) -> Result<(), AgfsError> {
        let normalized_path = normalize_path(path);

        // Remove from tree and get the mount point
        let mount_point = {
            let mut tree = self.mount_tree.write().unwrap();
            tree.remove(&normalized_path)
                .ok_or_else(|| AgfsError::NotFound(format!("mount point {}", normalized_path)))?
        };

        // Try to unwrap the Arc to get the MountPoint
        // This should succeed since we've removed it from the tree
        let mount_point = Arc::try_unwrap(mount_point)
            .map_err(|_| AgfsError::Internal("cannot unmount: plugin still referenced elsewhere".to_string()))?;

        // Shutdown the plugin (destructuring the MountPoint to get the plugin)
        let MountPoint { mut plugin, .. } = mount_point;
        plugin.shutdown()?;

        Ok(())
    }

    /// Get all mount points
    pub fn get_mounts(&self) -> Vec<Arc<MountPoint>> {
        let tree = self.mount_tree.read().unwrap();
        tree.iter().map(|(_, v)| v.clone()).collect()
    }

    /// Find the mount point for a given path
    ///
    /// Returns (mount_point, relative_path) if found.
    fn find_mount(&self, path: &str) -> Option<(Arc<MountPoint>, String)> {
        let normalized_path = normalize_path(path);
        let tree = self.mount_tree.read().unwrap();

        // Find the longest matching prefix
        let mut best_match: Option<(&str, &Arc<MountPoint>)> = None;

        for (key, value) in tree.iter() {
            // Check if the normalized path starts with this key
            if normalized_path.starts_with(key) {
                // Check if it's an exact match or a proper subpath
                if normalized_path.as_str() == key {
                    // Exact match
                    return Some((value.clone(), "/".to_string()));
                } else if normalized_path.len() > key.len() {
                    // Check if the next character is a path separator
                    let next_char = normalized_path.chars().nth(key.len());
                    if next_char == Some('/') {
                        // This is a valid subpath
                        match &best_match {
                            Some((existing_key, _)) if existing_key.len() > key.len() => {
                                // Keep the existing longer match
                            }
                            _ => {
                                best_match = Some((key, value));
                            }
                        }
                    }
                }
            }
        }

        // Special case: check for root mount
        if let Some(root) = tree.get("/") {
            if best_match.is_none() {
                return Some((root.clone(), normalized_path));
            }
        }

        best_match.map(|(key, value)| {
            let rel_path = if normalized_path == *key {
                "/".to_string()
            } else {
                normalized_path[key.len()..].to_string()
            };
            (value.clone(), rel_path)
        })
    }

    /// Resolve symlinks in a path
    fn resolve_path(&self, path: &str) -> Result<String, AgfsError> {
        let normalized = normalize_path(path);
        self.resolve_path_recursive(&normalized, 10)
    }

    /// Resolve symlinks recursively with loop detection
    fn resolve_path_recursive(&self, path: &str, max_depth: usize) -> Result<String, AgfsError> {
        if max_depth == 0 {
            return Err(AgfsError::Internal("too many levels of symbolic links".to_string()));
        }

        let normalized = normalize_path(path);

        // Check if this path is a symlink
        if let Some(target) = self.symlinks.get(&normalized) {
            let target = target.value();
            let resolved_target = if target.starts_with('/') {
                target.clone()
            } else {
                // Relative symlink - resolve relative to parent directory
                let parent = std::path::Path::new(&normalized)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".to_string());
                normalize_path(&format!("{}/{}", parent, target))
            };
            return self.resolve_path_recursive(&resolved_target, max_depth - 1);
        }

        // Resolve each path component
        if normalized == "/" {
            return Ok(normalized);
        }

        let parts: Vec<&str> = normalized
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut current_path = String::new();
        let mut resolved_parts = Vec::new();

        for (i, part) in parts.iter().enumerate() {
            current_path = format!("{}/{}", current_path, part);
            current_path = normalize_path(&current_path);

            // Check if this component is a symlink
            if let Some(target) = self.symlinks.get(&current_path) {
                let target = target.value();
                let resolved_target = if target.starts_with('/') {
                    target.clone()
                } else {
                    let parent = std::path::Path::new(&current_path)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "/".to_string());
                    normalize_path(&format!("{}/{}", parent, target))
                };

                // Recursively resolve the target
                let resolved = self.resolve_path_recursive(&resolved_target, max_depth - 1)?;

                // Build the remaining path
                let remaining = if i + 1 < parts.len() {
                    parts[i + 1..].join("/")
                } else {
                    String::new()
                };

                return if remaining.is_empty() {
                    Ok(resolved)
                } else {
                    Ok(normalize_path(&format!("{}/{}", resolved, remaining)))
                };
            }

            resolved_parts.push(*part);
        }

        Ok(normalize_path(&format!("/{}", resolved_parts.join("/"))))
    }
}

impl Default for MountableFS {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// FileSystem Implementation
// ============================================================================

impl FileSystem for MountableFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        let resolved = self.resolve_path(path)?;
        let (mount, rel_path) = self
            .find_mount(&resolved)
            .ok_or_else(|| AgfsError::PermissionDenied(format!("create {}", path)))?;
        mount.plugin.get_filesystem().create(&rel_path)
    }

    fn mkdir(&self, path: &str, perm: u32) -> Result<(), AgfsError> {
        let resolved = self.resolve_path(path)?;
        let (mount, rel_path) = self
            .find_mount(&resolved)
            .ok_or_else(|| AgfsError::PermissionDenied(format!("mkdir {}", path)))?;
        mount.plugin.get_filesystem().mkdir(&rel_path, perm)
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let normalized = normalize_path(path);

        // Check if it's a symlink first
        if self.symlinks.remove(&normalized).is_some() {
            return Ok(());
        }

        let resolved = self.resolve_path(path)?;
        let (mount, rel_path) = self
            .find_mount(&resolved)
            .ok_or_else(|| AgfsError::NotFound(format!("remove {}", path)))?;
        mount.plugin.get_filesystem().remove(&rel_path)
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        let (mount, rel_path) = self
            .find_mount(path)
            .ok_or_else(|| AgfsError::NotFound(format!("remove_all {}", path)))?;
        mount.plugin.get_filesystem().remove_all(&rel_path)
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let resolved = self.resolve_path(path)?;
        let (mount, rel_path) = self
            .find_mount(&resolved)
            .ok_or_else(|| AgfsError::NotFound(format!("read {}", path)))?;
        mount.plugin.get_filesystem().read(&rel_path, offset, size)
    }

    fn write(&self, path: &str, data: &[u8], offset: i64, flags: WriteFlag) -> Result<i64, AgfsError> {
        let resolved = self.resolve_path(path)?;
        let (mount, rel_path) = self
            .find_mount(&resolved)
            .ok_or_else(|| AgfsError::NotFound(format!("write {}", path)))?;
        mount.plugin.get_filesystem().write(&rel_path, data, offset, flags)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        let normalized = normalize_path(path);
        let resolved = self.resolve_path(&normalized)?;

        if let Some((mount, rel_path)) = self.find_mount(&resolved) {
            let mut infos = mount.plugin.get_filesystem().read_dir(&rel_path)?;

            // Add nested mount points
            let tree = self.mount_tree.read().unwrap();
            let prefix = if normalized.ends_with('/') {
                normalized.clone()
            } else {
                format!("{}/", normalized)
            };

            for (key, _value) in tree.iter() {
                if key.starts_with(&prefix) {
                    let suffix = &key[prefix.len()..];
                    if !suffix.contains('/') && !suffix.is_empty() {
                        // Direct child mount point
                        if !infos.iter().any(|info| info.name == suffix) {
                            infos.push(FileInfo {
                                name: suffix.to_string(),
                                size: 0,
                                mode: 0o755,
                                mod_time: Utc::now(),
                                is_dir: true,
                                is_symlink: false,
                                meta: MetaData {
                                    name: "rootfs".to_string(),
                                    r#type: META_VALUE_MOUNT_POINT.to_string(),
                                    content: Default::default(),
                                },
                            });
                        }
                    }
                }
            }

            // Add symlinks
            for link_entry in self.symlinks.iter() {
                let (link_path, _) = link_entry.pair();
                let link_parent = StdPath::new(link_path)
                    .parent()
                    .and_then(|p| p.to_str())
                    .map(normalize_path)
                    .unwrap_or_else(|| "/".to_string());

                if link_parent == normalized {
                    let link_name = StdPath::new(link_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    if !infos.iter().any(|info| info.name == link_name) {
                        infos.push(FileInfo {
                            name: link_name.to_string(),
                            size: 0,
                            mode: 0o777,
                            mod_time: Utc::now(),
                            is_dir: false,
                            is_symlink: true,
                            meta: MetaData {
                                name: "symlink".to_string(),
                                r#type: "symlink".to_string(),
                                content: Default::default(),
                            },
                        });
                    }
                }
            }

            return Ok(infos);
        }

        // Not in a mount, list virtual root
        let tree = self.mount_tree.read().unwrap();
        let mut infos = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let prefix = if normalized == "/" {
            String::new()
        } else if normalized.ends_with('/') {
            normalized.clone()
        } else {
            format!("{}/", normalized)
        };

        for (key, _) in tree.iter() {
            if key.starts_with(&prefix) {
                let suffix = &key[prefix.len()..];
                if let Some(next_part) = suffix.split('/').next() {
                    if !next_part.is_empty() && seen.insert(next_part) {
                        infos.push(FileInfo {
                            name: next_part.to_string(),
                            size: 0,
                            mode: 0o755,
                            mod_time: Utc::now(),
                            is_dir: true,
                            is_symlink: false,
                            meta: MetaData {
                                name: "rootfs".to_string(),
                                r#type: META_VALUE_MOUNT_POINT.to_string(),
                                content: Default::default(),
                            },
                        });
                    }
                }
            }
        }

        if !infos.is_empty() {
            Ok(infos)
        } else {
            Err(AgfsError::NotFound(format!("read_dir {}", path)))
        }
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        let normalized = normalize_path(path);

        // Root special case
        if normalized == "/" {
            return Ok(FileInfo {
                name: "/".to_string(),
                size: 0,
                mode: 0o755,
                mod_time: Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: MetaData {
                    name: META_VALUE_ROOT.to_string(),
                    r#type: META_VALUE_ROOT.to_string(),
                    content: Default::default(),
                },
            });
        }

        // Check if it's a symlink
        if let Some(target) = self.symlinks.get(&normalized) {
            let name = StdPath::new(&normalized)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Check if target is a directory
            let is_dir = if let Ok(resolved) = self.resolve_path(&normalized) {
                if let Ok(info) = self.stat_without_symlink(&resolved) {
                    info.is_dir
                } else {
                    false
                }
            } else {
                false
            };

            return Ok(FileInfo {
                name,
                size: target.len() as i64,
                mode: 0o777,
                mod_time: Utc::now(),
                is_dir,
                is_symlink: true,
                meta: MetaData {
                    name: "symlink".to_string(),
                    r#type: "symlink".to_string(),
                    content: Default::default(),
                },
            });
        }

        self.stat_without_symlink(&normalized)
    }

    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError> {
        let (old_mount, old_rel) = self
            .find_mount(old_path)
            .ok_or_else(|| AgfsError::NotFound(format!("rename {}", old_path)))?;
        let (new_mount, new_rel) = self
            .find_mount(new_path)
            .ok_or_else(|| AgfsError::NotFound(format!("rename {}", new_path)))?;

        if old_mount.path != new_mount.path {
            return Err(AgfsError::InvalidArgument("cannot rename across mounts".to_string()));
        }

        old_mount.plugin.get_filesystem().rename(&old_rel, &new_rel)
    }

    fn chmod(&self, path: &str, mode: u32) -> Result<(), AgfsError> {
        let resolved = self.resolve_path(path)?;
        let (mount, rel_path) = self
            .find_mount(&resolved)
            .ok_or_else(|| AgfsError::NotFound(format!("chmod {}", path)))?;
        mount.plugin.get_filesystem().chmod(&rel_path, mode)
    }

    fn open(&self, path: &str) -> Result<Box<dyn Read + Send>, AgfsError> {
        let resolved = self.resolve_path(path)?;
        let (mount, rel_path) = self
            .find_mount(&resolved)
            .ok_or_else(|| AgfsError::NotFound(format!("open {}", path)))?;
        mount.plugin.get_filesystem().open(&rel_path)
    }

    fn open_write(&self, path: &str) -> Result<Box<dyn Write + Send>, AgfsError> {
        let resolved = self.resolve_path(path)?;
        let (mount, rel_path) = self
            .find_mount(&resolved)
            .ok_or_else(|| AgfsError::NotFound(format!("open_write {}", path)))?;
        mount.plugin.get_filesystem().open_write(&rel_path)
    }
}

impl MountableFS {
    /// Stat without checking for symlinks (internal use to avoid recursion)
    fn stat_without_symlink(&self, path: &str) -> Result<FileInfo, AgfsError> {
        let resolved = self.resolve_path(path)?;
        if let Some((mount, rel_path)) = self.find_mount(&resolved) {
            let mut info = mount.plugin.get_filesystem().stat(&rel_path)?;

            // Fix name if querying mount point itself
            if path == mount.path && info.name == "/" {
                info.name = StdPath::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("/")
                    .to_string();
            }

            return Ok(info);
        }

        // Check if it's a parent directory of mount points
        let tree = self.mount_tree.read().unwrap();
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{}/", path)
        };

        for (key, _) in tree.iter() {
            if key.starts_with(&prefix) {
                let name = StdPath::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("/")
                    .to_string();

                return Ok(FileInfo {
                    name,
                    size: 0,
                    mode: 0o755,
                    mod_time: Utc::now(),
                    is_dir: true,
                    is_symlink: false,
                    meta: MetaData {
                        name: META_VALUE_MOUNT_POINT.to_string(),
                        r#type: META_VALUE_MOUNT_POINT.to_string(),
                        content: Default::default(),
                    },
                });
            }
        }

        Err(AgfsError::NotFound(format!("stat {}", path)))
    }
}

// ============================================================================
// Symlinker Implementation
// ============================================================================

impl Symlinker for MountableFS {
    fn symlink(&self, target: &str, link: &str) -> Result<(), AgfsError> {
        let normalized_link = normalize_path(link);

        // Check if link already exists
        if self.symlinks.contains_key(&normalized_link) {
            return Err(AgfsError::AlreadyExists(format!("symlink {}", normalized_link)));
        }

        // Check if a real file/directory exists
        if self.stat_without_symlink(&normalized_link).is_ok() {
            return Err(AgfsError::AlreadyExists(format!("path {}", normalized_link)));
        }

        // Verify parent directory exists
        let parent = StdPath::new(&normalized_link)
            .parent()
            .map(|p| normalize_path(p.to_str().unwrap_or("/")))
            .unwrap_or_else(|| "/".to_string());

        if parent != "/" && self.resolve_path(&parent).is_err() {
            return Err(AgfsError::Internal(format!("parent directory does not exist: {}", parent)));
        }

        // Store the symlink
        self.symlinks.insert(normalized_link.clone(), target.to_string());

        Ok(())
    }

    fn readlink(&self, link: &str) -> Result<String, AgfsError> {
        let normalized = normalize_path(link);
        self.symlinks
            .get(&normalized)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AgfsError::NotFound(format!("readlink {}", link)))
    }
}

// ============================================================================
// Toucher Implementation
// ============================================================================

impl Toucher for MountableFS {
    fn touch(&self, path: &str) -> Result<(), AgfsError> {
        let (mount, rel_path) = self
            .find_mount(path)
            .ok_or_else(|| AgfsError::NotFound(format!("touch {}", path)))?;

        let fs = mount.plugin.get_filesystem();

        // Fallback: read and write back (works for any filesystem)
        match fs.stat(&rel_path) {
            Ok(info) if !info.is_dir => {
                let data = fs.read(&rel_path, 0, -1)?;
                fs.write(&rel_path, &data, -1, WriteFlag::TRUNCATE)?;
                Ok(())
            }
            Ok(_) => Err(AgfsError::InvalidArgument("cannot touch directory".to_string())),
            Err(_) => {
                // File doesn't exist, create it
                fs.write(&rel_path, &[], -1, WriteFlag::CREATE)?;
                Ok(())
            }
        }
    }
}

// ============================================================================
// Truncater Implementation
// ============================================================================

impl Truncater for MountableFS {
    fn truncate(&self, path: &str, size: i64) -> Result<(), AgfsError> {
        let (mount, rel_path) = self
            .find_mount(path)
            .ok_or_else(|| AgfsError::NotFound(format!("truncate {}", path)))?;

        let fs = mount.plugin.get_filesystem();

        // Fallback: implement truncate using read/write
        if size == 0 {
            // Truncate to zero - write empty data
            fs.write(&rel_path, &[], 0, WriteFlag::TRUNCATE)?;
            Ok(())
        } else {
            // Get current file size
            let info = fs.stat(&rel_path)?;
            if info.size <= size {
                // Extend with zeros
                let current_data = fs.read(&rel_path, 0, -1)?;
                let mut new_data = vec![0u8; size as usize];
                new_data[..current_data.len()].copy_from_slice(&current_data);
                fs.write(&rel_path, &new_data, 0, WriteFlag::TRUNCATE)?;
            } else {
                // Truncate - read first `size` bytes
                let data = fs.read(&rel_path, 0, size)?;
                fs.write(&rel_path, &data, 0, WriteFlag::TRUNCATE)?;
            }
            Ok(())
        }
    }
}

// ============================================================================
// Streamer Implementation
// ============================================================================

impl Streamer for MountableFS {
    fn open_stream(&self, _path: &str) -> Result<Box<dyn StreamReader>, AgfsError> {
        // MountableFS itself doesn't support streaming by default
        // Individual plugins that implement Streamer will handle it
        Err(AgfsError::NotSupported)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Normalize a path to have a consistent format
///
/// - Removes duplicate slashes
/// - Removes trailing slashes (except for root)
/// - Ensures path starts with /
fn normalize_path(path: &str) -> String {
    let path = path.trim_start_matches("./")
        .trim_start_matches(".");

    if path.is_empty() || path == "/" {
        return "/".to_string();
    }

    let parts: Vec<&str> = path
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();

    if parts.is_empty() {
        return "/".to_string();
    }

    // Handle .. segments
    let mut result = Vec::new();
    for part in parts {
        if part == ".." {
            result.pop();
        } else {
            result.push(part);
        }
    }

    if result.is_empty() {
        return "/".to_string();
    }

    format!("/{}", result.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(""), "/");
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path("foo"), "/foo");
        assert_eq!(normalize_path("/foo"), "/foo");
        assert_eq!(normalize_path("/foo/"), "/foo");
        assert_eq!(normalize_path("/foo/bar"), "/foo/bar");
        assert_eq!(normalize_path("/foo//bar"), "/foo/bar");
        assert_eq!(normalize_path("/foo/./bar"), "/foo/bar");
        assert_eq!(normalize_path("/foo/../bar"), "/bar");
    }
}
