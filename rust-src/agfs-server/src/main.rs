//! AGFS Server - Main entry point
//!
//! This is the main entry point for the AGFS file system server.
//! It loads configuration, initializes plugins, and starts the HTTP server.

use agfs_server::{config::Config, mountablefs::MountableFS, plugins::{create_empty_plugin, memfs::create_memfs_plugin, localfs::create_localfs_plugin}, router};
use anyhow::{Context, Result};
use clap::Parser;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// AGFS Server command line arguments
#[derive(Parser, Debug)]
#[command(name = "agfs-server")]
#[command(about = "AGFS File System Server", long_about = None)]
struct Args {
    /// Configuration file path (YAML)
    #[arg(short, long, default_value = "agfs.yaml")]
    config: String,

    /// Listen address (overrides config file)
    #[arg(short, long)]
    listen: Option<String>,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&args.log_level));
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(env_filter)
        .init();

    info!("Starting AGFS Server");

    // Load configuration
    let config = load_config(&args.config).context("failed to load configuration")?;

    // Determine listen address
    let listen_addr = args.listen.unwrap_or_else(|| config.listen_addr.clone());
    info!("Listening on {}", listen_addr);

    // Create MountableFS
    let mfs = Arc::new(MountableFS::new());

    // Register builtin plugins
    register_builtin_plugins(&mfs);

    // Mount plugins from configuration
    mount_configured_plugins(&mfs, &config).context("failed to mount plugins")?;

    // Show mounted plugins
    let mounts = mfs.get_mounts();
    info!("Mounted {} plugin(s):", mounts.len());
    for mount in mounts {
        info!("  {} at {}", mount.plugin.name(), mount.path);
    }

    // Create router
    let app = router::create_router(mfs);

    // Start HTTP server
    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .context("failed to bind to address")?;

    info!("Server started successfully");

    axum::serve(listener, app)
        .await
        .context("server error")?;

    Ok(())
}

/// Load configuration from YAML file
fn load_config(path: &str) -> Result<Config> {
    // Try to read the config file
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let config: Config = serde_yaml::from_str(&content)
                .context("failed to parse YAML configuration")?;
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Config file doesn't exist, use default
            tracing::warn!(
                "Configuration file '{}' not found, using default configuration",
                path
            );
            Ok(Config::default())
        }
        Err(e) => Err(e).context("failed to read configuration file"),
    }
}

/// Register builtin plugins
fn register_builtin_plugins(mfs: &Arc<MountableFS>) {
    // Register empty plugin for testing
    mfs.register_plugin_factory("empty", create_empty_plugin);
    // Register memfs
    mfs.register_plugin_factory("memfs", create_memfs_plugin);
    // Register localfs
    mfs.register_plugin_factory("localfs", create_localfs_plugin);

    // TODO: Register more plugins as they are implemented
    // mfs.register_plugin_factory("kvfs", kvfs::create_kvfs_plugin);
    // mfs.register_plugin_factory("queuefs", queuefs::create_queuefs_plugin);
    // mfs.register_plugin_factory("serverinfofs", serverinfofs::create_serverinfofs_plugin);
    // etc.
}

/// Mount plugins from configuration
fn mount_configured_plugins(mfs: &Arc<MountableFS>, config: &Config) -> Result<()> {
    for plugin_config in &config.plugins {
        info!(
            "Mounting plugin '{}' at '{}'",
            plugin_config.name, plugin_config.mount
        );

        // Convert config to HashMap
        let config_map = serde_json::to_value(&plugin_config.config)
            .context("failed to convert plugin config")?
            .as_object()
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<serde_json::Map<String, serde_json::Value>>()
            })
            .unwrap_or_default();

        let config_map: std::collections::HashMap<String, serde_json::Value> =
            config_map.into_iter().collect();

        mfs.mount_plugin(&plugin_config.name, &plugin_config.mount, config_map)
            .with_context(|| {
                format!(
                    "failed to mount plugin '{}' at '{}'",
                    plugin_config.name, plugin_config.mount
                )
            })?;
    }
    Ok(())
}
