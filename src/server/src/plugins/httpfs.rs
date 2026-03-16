//! HttpFS - HTTP File Server for AGFS Paths
//!
//! Serves a AGFS mount path over HTTP, similar to 'python3 -m http.server'.

use ctxfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path as StdPath;
use std::sync::Arc;

/// MIME type mapping based on file extension
#[allow(dead_code)]
fn get_content_type(path: &str) -> &'static str {
    let path_lower = path.to_lowercase();

    // Special handling for README files
    let base_name = StdPath::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_uppercase();

    if base_name == "README" || base_name.starts_with("README.") {
        return "text/plain; charset=utf-8";
    }

    // Text formats
    if path_lower.ends_with(".txt") { return "text/plain; charset=utf-8"; }
    if path_lower.ends_with(".md") || path_lower.ends_with(".markdown") { return "text/markdown; charset=utf-8"; }
    if path_lower.ends_with(".json") { return "application/json; charset=utf-8"; }
    if path_lower.ends_with(".xml") { return "application/xml; charset=utf-8"; }
    if path_lower.ends_with(".html") || path_lower.ends_with(".htm") { return "text/html; charset=utf-8"; }
    if path_lower.ends_with(".css") { return "text/css; charset=utf-8"; }
    if path_lower.ends_with(".js") { return "application/javascript; charset=utf-8"; }
    if path_lower.ends_with(".yaml") || path_lower.ends_with(".yml") { return "text/yaml; charset=utf-8"; }
    if path_lower.ends_with(".log") { return "text/plain; charset=utf-8"; }
    if path_lower.ends_with(".csv") { return "text/csv; charset=utf-8"; }

    // Image formats
    if path_lower.ends_with(".png") { return "image/png"; }
    if path_lower.ends_with(".jpg") || path_lower.ends_with(".jpeg") { return "image/jpeg"; }
    if path_lower.ends_with(".gif") { return "image/gif"; }
    if path_lower.ends_with(".webp") { return "image/webp"; }
    if path_lower.ends_with(".svg") { return "image/svg+xml"; }
    if path_lower.ends_with(".ico") { return "image/x-icon"; }

    // Video formats
    if path_lower.ends_with(".mp4") { return "video/mp4"; }
    if path_lower.ends_with(".webm") { return "video/webm"; }
    if path_lower.ends_with(".ogg") { return "video/ogg"; }
    if path_lower.ends_with(".avi") { return "video/x-msvideo"; }
    if path_lower.ends_with(".mov") { return "video/quicktime"; }

    // Audio formats
    if path_lower.ends_with(".mp3") { return "audio/mpeg"; }
    if path_lower.ends_with(".wav") { return "audio/wav"; }
    if path_lower.ends_with(".m4a") { return "audio/mp4"; }
    if path_lower.ends_with(".flac") { return "audio/flac"; }

    // PDF
    if path_lower.ends_with(".pdf") { return "application/pdf"; }

    "application/octet-stream"
}

/// HTTPFS - Serves AGFS paths over HTTP
#[derive(Clone)]
pub struct HttpFS {
    agfs_path: String,
    http_host: String,
    http_port: String,
    status_path: String,
    #[allow(dead_code)]
    root_fs: Option<Arc<dyn FileSystem + Send + Sync>>,
    plugin_name: String,
    start_time: chrono::DateTime<chrono::Utc>,
    server_running: Arc<std::sync::atomic::AtomicBool>,
}

impl HttpFS {
    /// Create a new HttpFS instance
    pub fn new(
        agfs_path: impl Into<String>,
        http_host: impl Into<String>,
        http_port: impl Into<String>,
        status_path: impl Into<String>,
        root_fs: Option<Arc<dyn FileSystem + Send + Sync>>,
    ) -> Result<Self, AgfsError> {
        let agfs_path = agfs_path.into();
        if agfs_path.is_empty() {
            return Err(AgfsError::invalid_argument("agfs_path is required"));
        }

        let http_host = http_host.into();
        let http_port = http_port.into();
        let status_path = status_path.into();

        Ok(Self {
            agfs_path,
            http_host: if http_host.is_empty() { "0.0.0.0".to_string() } else { http_host },
            http_port: if http_port.is_empty() { "8000".to_string() } else { http_port },
            status_path,
            root_fs,
            plugin_name: "httpfs".to_string(),
            start_time: Utc::now(),
            server_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Start the HTTP server
    pub fn start_http_server(&self) -> Result<(), AgfsError> {
        // In full implementation, this would start an actual HTTP server
        // For now, mark as running
        self.server_running.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    /// Get the HTTP server URL
    pub fn server_url(&self) -> String {
        format!("http://{}:{}", self.http_host, self.http_port)
    }

    /// Get status information
    pub fn get_status_info(&self) -> String {
        let uptime = Utc::now() - self.start_time;
        let uptime_secs = uptime.num_seconds();

        format!(
            "HTTPFS Instance Status\n\
             ======================\n\
             \n\
             Virtual Path:    {}\n\
             AGFS Source Path: {}\n\
             HTTP Host:       {}\n\
             HTTP Port:       {}\n\
             HTTP Endpoint:   http://{}:{}\n\
             \n\
             Server Status:   {}\n\
             Uptime:          {} seconds\n\
             \n\
             Access this HTTP server:\n\
               Browser:       http://{}:{}/\n\
             ",
            self.status_path,
            self.agfs_path,
            self.http_host,
            self.http_port,
            self.http_host,
            self.http_port,
            if self.server_running.load(std::sync::atomic::Ordering::SeqCst) { "Running" } else { "Stopped" },
            uptime_secs,
            self.http_host,
            self.http_port,
        )
    }
}

impl FileSystem for HttpFS {
    fn create(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::invalid_argument("httpfs is read-only via filesystem interface, use HTTP to access files"))
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        Err(AgfsError::invalid_argument("httpfs is read-only via filesystem interface, use HTTP to access files"))
    }

    fn remove(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::invalid_argument("httpfs is read-only via filesystem interface, use HTTP to access files"))
    }

    fn remove_all(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::invalid_argument("httpfs is read-only via filesystem interface, use HTTP to access files"))
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        // Check if this is the virtual status file
        if path == "/" || path.is_empty() {
            let status_data = self.get_status_info().into_bytes();
            let offset = if offset < 0 { 0 } else { offset as usize };
            let size = if size < 0 { status_data.len() - offset } else { size as usize };

            if offset >= status_data.len() {
                return Ok(Vec::new());
            }

            let end = (offset + size).min(status_data.len());
            return Ok(status_data[offset..end].to_vec());
        }

        Err(AgfsError::invalid_argument("httpfs is read-only via filesystem interface, use HTTP to access files"))
    }

    fn write(&self, _path: &str, _data: &[u8], _offset: i64, _flags: WriteFlag) -> Result<i64, AgfsError> {
        Err(AgfsError::invalid_argument("httpfs is read-only via filesystem interface, use HTTP to access files"))
    }

    fn read_dir(&self, _path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        Err(AgfsError::invalid_argument("httpfs is read-only via filesystem interface, use HTTP to access files"))
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        // Check if this is the virtual status file
        if path == "/" || path.is_empty() {
            let status_data = self.get_status_info();
            return Ok(FileInfo {
                name: "status".to_string(),
                size: status_data.len() as i64,
                mode: 0o444, // Read-only
                mod_time: self.start_time,
                is_dir: false,
                is_symlink: false,
                meta: MetaData {
                    name: self.plugin_name.clone(),
                    r#type: "virtual".to_string(),
                    content: {
                        let mut map = HashMap::new();
                        map.insert("description".to_string(), "HTTP file server status".to_string());
                        map
                    },
                },
            });
        }

        Err(AgfsError::not_found(path))
    }

    fn rename(&self, _old_path: &str, _new_path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::invalid_argument("httpfs is read-only via filesystem interface, use HTTP to access files"))
    }

    fn chmod(&self, _path: &str, _mode: u32) -> Result<(), AgfsError> {
        Err(AgfsError::invalid_argument("httpfs is read-only via filesystem interface, use HTTP to access files"))
    }

    fn open(&self, path: &str) -> Result<Box<dyn std::io::Read + Send>, AgfsError> {
        let data = self.read(path, 0, -1)?;
        Ok(Box::new(std::io::Cursor::new(data)))
    }

    fn open_write(&self, _path: &str) -> Result<Box<dyn std::io::Write + Send>, AgfsError> {
        Err(AgfsError::invalid_argument("httpfs is read-only via filesystem interface, use HTTP to access files"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_httpfs_create() {
        let fs = HttpFS::new("/memfs", "localhost", "8000", "/httpfs", None).unwrap();
        assert_eq!(fs.agfs_path, "/memfs");
        assert_eq!(fs.http_host, "localhost");
        assert_eq!(fs.http_port, "8000");
    }

    #[test]
    fn test_httpfs_default_values() {
        let fs = HttpFS::new("/memfs", "", "", "/httpfs", None).unwrap();
        assert_eq!(fs.http_host, "0.0.0.0");
        assert_eq!(fs.http_port, "8000");
    }

    #[test]
    fn test_httpfs_invalid_agfs_path() {
        let result = HttpFS::new("", "localhost", "8000", "/httpfs", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_content_type() {
        assert_eq!(get_content_type("test.txt"), "text/plain; charset=utf-8");
        assert_eq!(get_content_type("test.json"), "application/json; charset=utf-8");
        assert_eq!(get_content_type("test.png"), "image/png");
        assert_eq!(get_content_type("test.pdf"), "application/pdf");
        assert_eq!(get_content_type("test.unknown"), "application/octet-stream");
    }

    #[test]
    fn test_httpfs_read_status() {
        let fs = HttpFS::new("/memfs", "localhost", "8000", "/httpfs", None).unwrap();
        let data = fs.read("/", 0, -1).unwrap();
        assert!(!data.is_empty());
        let status = String::from_utf8_lossy(&data);
        assert!(status.contains("HTTPFS Instance Status"));
    }
}
