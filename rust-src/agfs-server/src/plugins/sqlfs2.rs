//! SqlFS2 - Improved SQL File System
//!
//! Plan 9 style interface with session management.

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

/// Query session
#[derive(Debug, Clone)]
struct Session {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    query: String,
    result: Vec<Vec<String>>,
    #[allow(dead_code)]
    created_at: chrono::DateTime<chrono::Utc>,
}

/// SQL FS 2 with Plan 9 style interface
#[derive(Debug, Clone)]
pub struct SqlFS2 {
    sessions: Arc<DashMap<String, Session>>,
    session_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl SqlFS2 {
    /// Create a new SqlFS2 instance
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            session_counter: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Generate new session ID
    fn generate_session_id(&self) -> String {
        format!("{}", self.session_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }

    /// Execute a query
    pub fn execute_query(&self, query: &str) -> Result<Vec<Vec<String>>, AgfsError> {
        let session_id = self.generate_session_id();

        // In full implementation, would execute SQL query here
        // For now, return placeholder result
        let result = vec![vec!["id".to_string(), "value".to_string()]];

        self.sessions.insert(session_id.clone(), Session {
            id: session_id.clone(),
            query: query.to_string(),
            result: result.clone(),
            created_at: Utc::now(),
        });

        Ok(result)
    }
}

impl Default for SqlFS2 {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for SqlFS2 {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        match path {
            "/ctl" => Ok(()), // Control file
            _ => Err(AgfsError::NotSupported),
        }
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        Ok(())
    }

    fn remove(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn remove_all(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn read(&self, path: &str, _offset: i64, _size: i64) -> Result<Vec<u8>, AgfsError> {
        match path {
            "/ctl" => Ok(b"Write query here to get session ID\n".to_vec()),
            _ => {
                // Check if it's a result file
                if let Some(session_id) = path.strip_prefix("/result/") {
                    if let Some(session) = self.sessions.get(session_id) {
                        let rows = &session.result;
                        let output = rows.iter()
                            .map(|row| row.join(","))
                            .collect::<Vec<_>>()
                            .join("\n");
                        return Ok(output.into_bytes());
                    }
                }
                Err(AgfsError::not_found(path))
            }
        }
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, _flags: WriteFlag) -> Result<i64, AgfsError> {
        if path == "/ctl" {
            let query = std::str::from_utf8(data)
                .map_err(|_| AgfsError::invalid_argument("invalid UTF-8"))?;

            self.execute_query(query)?;

            // Return session ID as the write result
            let session_id = self.sessions.len().to_string();
            Ok(session_id.len() as i64)
        } else {
            Err(AgfsError::NotSupported)
        }
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        if path == "/" || path.is_empty() {
            let mut files = vec![
                FileInfo {
                    name: "ctl".to_string(),
                    size: 0,
                    mode: 0o644,
                    mod_time: Utc::now(),
                    is_dir: false,
                    is_symlink: false,
                    meta: MetaData::with_type("control"),
                },
            ];

            // Add result files for each session
            for session_id in self.sessions.iter().map(|e| e.key().clone()) {
                files.push(FileInfo {
                    name: format!("result/{}", session_id),
                    size: 0,
                    mode: 0o444,
                    mod_time: Utc::now(),
                    is_dir: false,
                    is_symlink: false,
                    meta: MetaData::with_type("result"),
                });
            }

            Ok(files)
        } else {
            Err(AgfsError::not_found(path))
        }
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        match path {
            "/" | "" => Ok(FileInfo {
                name: String::new(),
                size: self.sessions.len() as i64,
                mode: 0o555,
                mod_time: Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: MetaData::default(),
            }),
            "/ctl" => Ok(FileInfo {
                name: "ctl".to_string(),
                size: 0,
                mode: 0o644,
                mod_time: Utc::now(),
                is_dir: false,
                is_symlink: false,
                meta: MetaData::with_type("control"),
            }),
            _ => Err(AgfsError::not_found(path)),
        }
    }

    fn rename(&self, _old_path: &str, _new_path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn chmod(&self, _path: &str, _mode: u32) -> Result<(), AgfsError> {
        Ok(())
    }

    fn open(&self, path: &str) -> Result<Box<dyn std::io::Read + Send>, AgfsError> {
        let data = self.read(path, 0, -1)?;
        Ok(Box::new(std::io::Cursor::new(data)))
    }

    fn open_write(&self, _path: &str) -> Result<Box<dyn std::io::Write + Send>, AgfsError> {
        Err(AgfsError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlfs2_ctl() {
        let fs = SqlFS2::new();

        // Write query to ctl
        fs.write("/ctl", b"SELECT * FROM test", 0, WriteFlag::NONE).unwrap();

        // Read result files
        let files = fs.read_dir("/").unwrap();
        assert!(files.iter().any(|f| f.name.starts_with("result/")));
    }

    #[test]
    fn test_sqlfs2_read_result() {
        let fs = SqlFS2::new();

        // Create a session by writing to ctl
        fs.write("/ctl", b"SELECT 1", 0, WriteFlag::NONE).unwrap();

        // List to find result file
        let files = fs.read_dir("/").unwrap();
        let result_file = files.iter()
            .find(|f| f.name.starts_with("result/"))
            .map(|f| f.name.clone());

        if let Some(result_file) = result_file {
            let data = fs.read(&format!("/{}", result_file), 0, -1).unwrap();
            assert!(!data.is_empty());
        }
    }
}
