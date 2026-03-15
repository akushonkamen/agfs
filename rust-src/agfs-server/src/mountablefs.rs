//! Mountable file system - routes requests to plugins

// TODO: Remove this allow once full implementation is complete
#![allow(missing_docs)]

use std::collections::HashMap;

/// MountableFS - routes file system requests to mounted plugins
pub struct MountableFS {
    plugins: HashMap<String, String>,
}

impl MountableFS {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, mount: String) {
        self.plugins.insert(name, mount);
    }
}

impl Default for MountableFS {
    fn default() -> Self {
        Self::new()
    }
}
