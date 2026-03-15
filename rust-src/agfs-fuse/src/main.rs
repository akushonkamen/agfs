//! AGFS FUSE - Mount AGFS servers as local filesystems
//!
//! This is the CLI entry point for the FUSE client.

#[cfg(target_os = "linux")]
use std::path::Path;

#[cfg(not(target_os = "linux"))]
compile_error!("AGFS FUSE is only supported on Linux");

use agfs_fuse::fusefs::Config;
use clap::{Parser, ValueEnum};
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::time::Duration;
use tokio::runtime::Runtime;

/// AGFS FUSE - Mount AGFS servers as local filesystems
#[derive(Parser, Debug)]
#[command(name = "agfs-fuse")]
#[command(author = "AGFS Team")]
#[command(version = "0.1.0")]
#[command(about = "Mount AGFS server as a FUSE filesystem", long_about = None)]
struct Args {
    /// AGFS server URL
    #[arg(long = "agfs-server-url")]
    #[arg(default_value = "http://localhost:8080/api/v1")]
    #[arg(env = "AGFS_SERVER_URL")]
    server_url: String,

    /// Mount point directory
    #[arg(long = "mount")]
    #[arg(short = 'm')]
    mount_point: String,

    /// Cache TTL duration (e.g., "5s", "30s", "1m")
    #[arg(long = "cache-ttl")]
    #[arg(default_value = "5s")]
    cache_ttl: String,

    /// Log level
    #[arg(long = "log-level")]
    #[arg(default_value = "info")]
    #[arg(value_enum)]
    log_level: LogLevel,

    /// Enable debug output
    #[arg(long = "debug")]
    debug: bool,

    /// Allow other users to access the mount
    #[arg(long = "allow-other")]
    allow_other: bool,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn to_tracing_level(&self) -> tracing::Level {
        match self {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim().to_lowercase();

    // First try to parse as plain number (default to seconds)
    if let Ok(secs) = s.parse::<u64>() {
        return Ok(Duration::from_secs(secs));
    }

    // Parse duration string like "5s", "100ms", "1m", "2h"
    // Order matters: check longer/more specific suffixes first

    // Milliseconds (must be before "s" and "m")
    if let Some(suffix) = s.strip_suffix("ms") {
        let num_str = suffix.trim();
        let millis: u64 = num_str
            .parse()
            .map_err(|_| format!("Invalid duration: {}", s))?;
        return Ok(Duration::from_millis(millis));
    }

    // Seconds with "sec" suffix
    if s.ends_with("sec") {
        let num_str = s.trim_end_matches("sec").trim();
        let secs: u64 = num_str
            .parse()
            .map_err(|_| format!("Invalid duration: {}", s))?;
        return Ok(Duration::from_secs(secs));
    }

    // Seconds with "s" suffix (must be after "ms")
    if let Some(suffix) = s.strip_suffix("s") {
        if !suffix.is_empty() {
            let secs: u64 = suffix
                .parse()
                .map_err(|_| format!("Invalid duration: {}", s))?;
            return Ok(Duration::from_secs(secs));
        }
    }

    // Minutes (must be after "ms")
    if let Some(suffix) = s.strip_suffix("m") {
        let num_str = suffix.trim();
        let mins: u64 = num_str
            .parse()
            .map_err(|_| format!("Invalid duration: {}", s))?;
        return Ok(Duration::from_secs(mins * 60));
    }

    // Hours
    if let Some(suffix) = s.strip_suffix("h") {
        let num_str = suffix.trim();
        let hours: u64 = num_str
            .parse()
            .map_err(|_| format!("Invalid duration: {}", s))?;
        return Ok(Duration::from_secs(hours * 3600));
    }

    // If nothing matched, try parsing as plain number (seconds)
    let secs: u64 = s.parse().map_err(|_| format!("Invalid duration: {}", s))?;
    Ok(Duration::from_secs(secs))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize tracing
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(args.log_level.to_tracing_level())
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(args.debug)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    // Parse cache TTL
    let cache_ttl = parse_duration(&args.cache_ttl)?;

    // Validate mount point
    let mount_path = Path::new(&args.mount_point);
    if !mount_path.exists() {
        eprintln!("Error: Mount point '{}' does not exist", args.mount_point);
        eprintln!("Create it with: mkdir -p {}", args.mount_point);
        std::process::exit(1);
    }

    if !mount_path.is_dir() {
        eprintln!("Error: Mount point '{}' is not a directory", args.mount_point);
        std::process::exit(1);
    }

    // Create filesystem configuration
    let config = Config {
        server_url: args.server_url.clone(),
        cache_ttl,
        debug: args.debug,
    };

    // Create tokio runtime
    let rt = Runtime::new()?;

    // Create filesystem
    let fs = rt.block_on(async {
        agfs_fuse::fusefs::AGFSFS::new(config)
    })?;

    tracing::info!("AGFS FUSE v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Server: {}", args.server_url);
    tracing::info!("Mount point: {}", args.mount_point);
    tracing::info!("Cache TTL: {:?}", cache_ttl);
    tracing::info!("Press Ctrl+C to unmount");

    // Setup FUSE mount options
    let mut mount_options = fuse3::MountOptions::default();
    mount_options.fs_name("agfs");
    if args.allow_other {
        mount_options.allow_other(true);
    }

    // Create session and mount
    let session = fuse3::raw::Session::new(mount_options);

    // Run the mount in background
    let mount_handle = rt.block_on(async {
        // For unprivileged mount
        #[cfg(feature = "unprivileged")]
        {
            session.mount_with_unprivileged(fs, &args.mount_point).await
        }
        // For privileged mount
        #[cfg(not(feature = "unprivileged"))]
        {
            session.mount(fs, &args.mount_point).await
        }
    })?;

    // Setup signal handlers for graceful shutdown
    let mut signals = Signals::new([SIGINT, signal_hook::consts::SIGTERM])?;

    // Wait for signal
    for _ in signals.forever() {
        tracing::info!("Received shutdown signal, unmounting...");

        // Unmount using the mount handle
        rt.block_on(async {
            let _ = mount_handle.unmount().await;
        });

        tracing::info!("AGFS unmounted successfully");
        break;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("5").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("100ms").unwrap(), Duration::from_millis(100));
        assert_eq!(parse_duration("1m").unwrap(), Duration::from_secs(60));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
    }

    #[test]
    fn test_parse_duration_case_insensitive() {
        assert_eq!(parse_duration("5S").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("100MS").unwrap(), Duration::from_millis(100));
        assert_eq!(parse_duration("1M").unwrap(), Duration::from_secs(60));
    }
}
