//! AGFS server configuration

// TODO: Remove this allow once full implementation is complete
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub listen_addr: String,
    pub plugins: Vec<PluginConfig>,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub mount: String,
    #[serde(flatten)]
    pub config: YamlValue,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8080".to_string(),
            plugins: Vec::new(),
        }
    }
}
