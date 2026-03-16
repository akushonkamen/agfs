//! SqlFS2 - Improved SQL File System
//!
//! Plan 9 style interface with session management and real SQL queries.
//! Uses SQLite as the default backend.

use ctxfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use dashmap::DashMap;
use serde_json::{json, Value};
use sqlx::{Column, Row, SqlitePool};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

/// Query session with real SQL execution
#[derive(Debug, Clone)]
struct Session {
    id: String,
    db_name: String,
    table_name: String,
    query: String,
    result: Value,
    row_count: i64,
    last_error: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_access: chrono::DateTime<chrono::Utc>,
}

impl Session {
    /// Update last access time
    fn touch(&mut self) {
        self.last_access = Utc::now();
    }

    /// Check if session is expired
    fn is_expired(&self, timeout: Duration) -> bool {
        Utc::now().signed_duration_since(self.last_access).num_seconds() > timeout.as_secs() as i64
    }
}

/// Session manager with timeout cleanup
#[derive(Debug)]
struct SessionManager {
    sessions: DashMap<String, Session>,
    next_id: Arc<std::sync::atomic::AtomicU64>,
    timeout: Duration,
    cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SessionManager {
    /// Create a new session manager
    fn new(timeout: Duration) -> Self {
        let mut manager = Self {
            sessions: DashMap::new(),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            timeout,
            cleanup_handle: None,
        };
        manager.start_cleanup_task();
        manager
    }

    /// Start background cleanup task
    fn start_cleanup_task(&mut self) {
        let sessions = self.sessions.clone();
        let timeout = self.timeout;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(timeout / 2);
            loop {
                interval.tick().await;
                let _now = Utc::now();
                sessions.retain(|_, session| {
                    !session.is_expired(timeout)
                });
            }
        });

        self.cleanup_handle = Some(handle);
    }

    /// Generate new session ID
    fn generate_id(&self) -> String {
        format!("{}", self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }

    /// Create a new session
    fn create_session(&self, db_name: String, table_name: String, query: String, result: Value, row_count: i64) -> String {
        let id = self.generate_id();
        let now = Utc::now();

        let session = Session {
            id: id.clone(),
            db_name,
            table_name,
            query,
            result,
            row_count,
            last_error: None,
            created_at: now,
            last_access: now,
        };

        self.sessions.insert(id.clone(), session);
        id
    }

    /// Get a session and update last access
    fn get_session(&self, id: &str) -> Option<Session> {
        let mut session = self.sessions.get(id)?.clone();
        session.touch();
        Some(session)
    }

    /// Remove a session
    fn remove_session(&self, id: &str) -> bool {
        self.sessions.remove(id).is_some()
    }

    /// List all session IDs
    fn list_sessions(&self) -> Vec<String> {
        self.sessions.iter().map(|e| e.key().clone()).collect()
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
    }
}

/// SQL FS 2 with Plan 9 style interface and real database backend
#[derive(Debug, Clone)]
pub struct SqlFS2 {
    pool: Arc<SqlitePool>,
    default_database: String,
    default_table: String,
    sessions: Arc<SessionManager>,
    session_timeout: Duration,
}

impl SqlFS2 {
    /// Create a new SqlFS2 instance from connection string
    ///
    /// Connection string format:
    /// - SQLite: `sqlite:path/to/database.db` or `sqlite::memory:`
    pub async fn new(connection_string: &str) -> Result<Self, AgfsError> {
        Self::with_timeout(connection_string, Duration::from_secs(300)).await
    }

    /// Create a new SqlFS2 instance with custom session timeout
    pub async fn with_timeout(connection_string: &str, timeout: Duration) -> Result<Self, AgfsError> {
        // Convert to sqlite: format if needed
        let conn_str = if connection_string.starts_with("sqlite:") {
            connection_string.to_string()
        } else if connection_string.contains(":memory:") {
            format!("sqlite:{}", connection_string)
        } else {
            format!("sqlite:{}", connection_string)
        };

        // Create connection pool
        let pool = SqlitePool::connect(&conn_str)
            .await
            .map_err(|e| AgfsError::internal(format!("Failed to connect to database: {}", e)))?;

        // Extract database name from connection string
        let default_database = extract_database_name(&conn_str);
        let default_table = "default_table".to_string();

        let sessions = Arc::new(SessionManager::new(timeout));

        Ok(Self {
            pool: Arc::new(pool),
            default_database,
            default_table,
            sessions,
            session_timeout: timeout,
        })
    }

    /// Execute a SQL query and create a session
    pub async fn execute_query(&self, query: &str, db_name: Option<&str>, table_name: Option<&str>) -> Result<String, AgfsError> {
        let db_name = db_name.unwrap_or(&self.default_database);
        let table_name = table_name.unwrap_or(&self.default_table);

        // Execute the query
        let result = self.execute_query_inner(query).await?;

        // Create session with result
        let session_id = self.sessions.create_session(
            db_name.to_string(),
            table_name.to_string(),
            query.to_string(),
            result.0,
            result.1,
        );

        Ok(session_id)
    }

    /// Inner query execution
    async fn execute_query_inner(&self, query: &str) -> Result<(Value, i64), AgfsError> {
        let query = query.trim();

        // Handle different query types
        let upper = query.to_uppercase();
        if upper.starts_with("SELECT") || upper.starts_with("SHOW") || upper.starts_with("DESCRIBE") || upper.starts_with("EXPLAIN") || upper.starts_with("PRAGMA") {
            // Query that returns rows
            self.execute_select(query).await
        } else {
            // Query that returns affected rows
            self.execute_update(query).await
        }
    }

    /// Execute a SELECT query
    async fn execute_select(&self, query: &str) -> Result<(Value, i64), AgfsError> {
        let rows = sqlx::query(query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;

        let mut result = json!({
            "columns": [],
            "rows": [],
        });

        if let Some(first_row) = rows.first() {
            // Get column names
            let columns = first_row.columns();
            let column_names: Vec<String> = columns.iter().map(|c| c.name().to_string()).collect();
            result["columns"] = json!(column_names);

            // Build rows
            let mut row_data = Vec::new();
            for row in &rows {
                let mut row_obj = serde_json::Map::new();
                for col in columns {
                    let name = col.name().to_string();
                    // Try to get value as string
                    let value: Option<String> = row.try_get(name.as_str()).ok();
                    row_obj.insert(name, json!(value.unwrap_or_else(|| "NULL".to_string())));
                }
                row_data.push(json!(row_obj));
            }
            result["rows"] = json!(row_data);
        }

        Ok((result, rows.len() as i64))
    }

    /// Execute an UPDATE/INSERT/DELETE query
    async fn execute_update(&self, query: &str) -> Result<(Value, i64), AgfsError> {
        let result = sqlx::query(query)
            .execute(&*self.pool)
            .await
            .map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;

        let affected = result.rows_affected() as i64;
        let json_result = json!({
            "affected_rows": affected,
        });

        Ok((json_result, affected))
    }

    /// Get session result as JSON string
    pub fn get_session_result(&self, session_id: &str) -> Result<String, AgfsError> {
        if let Some(session) = self.sessions.get_session(session_id) {
            Ok(session.result.to_string())
        } else {
            Err(AgfsError::not_found(session_id))
        }
    }

    /// Get session info
    pub fn get_session_info(&self, session_id: &str) -> Result<Value, AgfsError> {
        if let Some(session) = self.sessions.get_session(session_id) {
            Ok(json!({
                "id": session.id,
                "database": session.db_name,
                "table": session.table_name,
                "query": session.query,
                "row_count": session.row_count,
                "created_at": session.created_at.to_rfc3339(),
                "last_access": session.last_access.to_rfc3339(),
                "error": session.last_error,
            }))
        } else {
            Err(AgfsError::not_found(session_id))
        }
    }

    /// Helper to execute query with a pool reference (for use in spawned threads)
    /// Returns (session_id, db_name, table_name, query, result, row_count)
    async fn execute_query_with_pool(
        pool: &SqlitePool,
        query: &str,
        db_name: Option<&str>,
        table_name: Option<&str>,
    ) -> Result<(String, String, String, String, Value, i64), AgfsError> {
        let db_name = db_name.unwrap_or("sqlite").to_string();
        let table_name = table_name.unwrap_or("table").to_string();
        let query_string = query.to_string();

        // Execute the query
        let (result, row_count) = Self::execute_query_inner_with_pool(pool, query).await?;

        // Create a simple session ID (just a hash)
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        let session_id = format!("{:x}", hasher.finish());

        Ok((session_id, db_name, table_name, query_string, result, row_count))
    }

    /// Execute SELECT query
    async fn execute_query_inner_with_pool(pool: &SqlitePool, query: &str) -> Result<(Value, i64), AgfsError> {
        let query = query.trim();
        let upper = query.to_uppercase();

        if upper.starts_with("SELECT") || upper.starts_with("SHOW")
            || upper.starts_with("DESCRIBE") || upper.starts_with("EXPLAIN")
            || upper.starts_with("PRAGMA") {
            // Query that returns rows
            Self::execute_select_with_pool(pool, query).await
        } else {
            // Query that returns affected rows
            Self::execute_update_with_pool(pool, query).await
        }
    }

    /// Execute SELECT query
    async fn execute_select_with_pool(pool: &SqlitePool, query: &str) -> Result<(Value, i64), AgfsError> {
        let rows = sqlx::query(query).fetch_all(pool).await
            .map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;

        if rows.is_empty() {
            return Ok((json!([]), 0));
        }

        let mut result = Vec::new();
        for row in rows {
            let mut obj = serde_json::Map::new();
            // Use column names
            if let Some(col) = row.columns().first() {
                let name = col.name();
                if let Ok(val) = row.try_get::<String, _>(name) {
                    obj.insert(name.to_string(), json!(val));
                } else if let Ok(val) = row.try_get::<i64, _>(name) {
                    obj.insert(name.to_string(), json!(val));
                } else if let Ok(val) = row.try_get::<f64, _>(name) {
                    obj.insert(name.to_string(), json!(val));
                }
            }
            if !obj.is_empty() {
                result.push(obj);
            }
        }

        Ok((json!(result), result.len() as i64))
    }

    /// Execute UPDATE/INSERT/DELETE query
    async fn execute_update_with_pool(pool: &SqlitePool, query: &str) -> Result<(Value, i64), AgfsError> {
        let result = sqlx::query(query).execute(pool).await
            .map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;

        let affected = result.rows_affected();
        Ok((json!({"affected_rows": affected}), affected as i64))
    }
}

/// Extract database name from connection string
fn extract_database_name(conn_str: &str) -> String {
    if let Some(path) = conn_str.strip_prefix("sqlite:") {
        // Handle SQLite special paths
        if path == ":memory:" || path == "::memory:" {
            return "memory".to_string();
        }
        if let Some(name) = std::path::Path::new(path).file_stem() {
            return name.to_string_lossy().to_string();
        }
        "sqlite".to_string()
    } else {
        "sqlite".to_string()
    }
}

impl Default for SqlFS2 {
    fn default() -> Self {
        // Create a default instance with in-memory SQLite
        let runtime = tokio::runtime::Handle::try_current();
        let pool = if let Ok(handle) = runtime {
            handle.block_on(async {
                SqlitePool::connect("sqlite::memory:").await
            })
        } else {
            // Create a new runtime if none exists
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                SqlitePool::connect("sqlite::memory:").await
            })
        };

        Self {
            pool: Arc::new(pool.expect("Failed to create default pool")),
            default_database: "default".to_string(),
            default_table: "default".to_string(),
            sessions: Arc::new(SessionManager::new(Duration::from_secs(300))),
            session_timeout: Duration::from_secs(300),
        }
    }
}

impl FileSystem for SqlFS2 {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        match path {
            "/ctl" => Ok(()), // Control file is virtual
            _ => Err(AgfsError::NotSupported),
        }
    }

    fn mkdir(&self, _path: &str, _perm: u32) -> Result<(), AgfsError> {
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        // Allow removing session results
        if path.starts_with("/result/") {
            let session_id = path.strip_prefix("/result/").unwrap_or("");
            if self.sessions.remove_session(session_id) {
                return Ok(());
            }
        }
        Err(AgfsError::NotSupported)
    }

    fn remove_all(&self, _path: &str) -> Result<(), AgfsError> {
        Err(AgfsError::NotSupported)
    }

    fn read(&self, path: &str, _offset: i64, _size: i64) -> Result<Vec<u8>, AgfsError> {
        match path {
            "/ctl" => Ok(b"Write query here to get session ID\nFormat: database/table/query\n".to_vec()),
            "/sessions" => {
                let sessions = self.sessions.list_sessions();
                let json = json!({
                    "sessions": sessions,
                    "count": sessions.len(),
                });
                Ok(json.to_string().into_bytes())
            }
            _ => {
                // Check if it's a result file
                if let Some(session_id) = path.strip_prefix("/result/") {
                    let result = self.get_session_result(session_id)?;
                    return Ok(result.into_bytes());
                }
                // Check if it's a session info file
                if let Some(session_id) = path.strip_prefix("/info/") {
                    let info = self.get_session_info(session_id)?;
                    return Ok(info.to_string().into_bytes());
                }
                Err(AgfsError::not_found(path))
            }
        }
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, _flags: WriteFlag) -> Result<i64, AgfsError> {
        if path == "/ctl" {
            let query = std::str::from_utf8(data)
                .map_err(|_| AgfsError::invalid_argument("invalid UTF-8"))?;

            // Parse query format: database/table/QUERY or just QUERY
            let (db_name, table_name, actual_query) = if query.contains('/') {
                let parts: Vec<&str> = query.splitn(3, '/').collect();
                if parts.len() == 3 {
                    (Some(parts[0].to_string()), Some(parts[1].to_string()), parts[2].to_string())
                } else {
                    (None, None, query.to_string())
                }
            } else {
                (None, None, query.to_string())
            };

            // Clone for the thread
            let pool = self.pool.clone();
            let sessions = self.sessions.clone();

            let (session_id, db_name, table_name, query_str, result, row_count) = std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AgfsError::internal(format!("failed to create runtime: {}", e)))?;
                rt.block_on(async move {
                    Self::execute_query_with_pool(&pool, &actual_query, db_name.as_deref(), table_name.as_deref()).await
                })
            })
            .join()
            .map_err(|_| AgfsError::internal("task join failed"))??;

            // Create the session in the session manager
            sessions.create_session(db_name, table_name, query_str, result, row_count);

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
                FileInfo {
                    name: "sessions".to_string(),
                    size: 0,
                    mode: 0o444,
                    mod_time: Utc::now(),
                    is_dir: false,
                    is_symlink: false,
                    meta: MetaData::with_type("session-list"),
                },
            ];

            // Add result files for each session
            for session_id in self.sessions.list_sessions() {
                files.push(FileInfo {
                    name: format!("result/{}", session_id),
                    size: 0,
                    mode: 0o444,
                    mod_time: Utc::now(),
                    is_dir: false,
                    is_symlink: false,
                    meta: MetaData::with_type("result"),
                });
                files.push(FileInfo {
                    name: format!("info/{}", session_id),
                    size: 0,
                    mode: 0o444,
                    mod_time: Utc::now(),
                    is_dir: false,
                    is_symlink: false,
                    meta: MetaData::with_type("info"),
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
                size: self.sessions.list_sessions().len() as i64,
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
            "/sessions" => Ok(FileInfo {
                name: "sessions".to_string(),
                size: 0,
                mode: 0o444,
                mod_time: Utc::now(),
                is_dir: false,
                is_symlink: false,
                meta: MetaData::with_type("session-list"),
            }),
            _ => {
                if path.starts_with("/result/") || path.starts_with("/info/") {
                    Ok(FileInfo {
                        name: path.trim_start_matches('/').to_string(),
                        size: 0,
                        mode: 0o444,
                        mod_time: Utc::now(),
                        is_dir: false,
                        is_symlink: false,
                        meta: MetaData::default(),
                    })
                } else {
                    Err(AgfsError::not_found(path))
                }
            }
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

    #[tokio::test]
    async fn test_sqlfs2_create_table_and_query() {
        let fs = SqlFS2::new("sqlite::memory:").await.unwrap();

        // Create a test table
        let create_query = "CREATE TABLE test_users (id INTEGER PRIMARY KEY, name TEXT)";
        let session_id = fs.execute_query(create_query, Some("test"), Some("test_users")).await.unwrap();
        assert!(!session_id.is_empty());

        // Insert some data
        let insert_query = "INSERT INTO test_users (id, name) VALUES (1, 'Alice'), (2, 'Bob')";
        let session_id = fs.execute_query(insert_query, Some("test"), Some("test_users")).await.unwrap();
        let info = fs.get_session_info(&session_id).unwrap();
        assert_eq!(info["row_count"], 2);

        // Query the data
        let select_query = "SELECT * FROM test_users";
        let session_id = fs.execute_query(select_query, Some("test"), Some("test_users")).await.unwrap();
        let result = fs.get_session_result(&session_id).unwrap();
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
    }

    #[tokio::test]
    async fn test_sqlfs2_ctl() {
        let fs = SqlFS2::new("sqlite::memory:").await.unwrap();

        // Write query to ctl
        fs.write("/ctl", b"SELECT 1 as num", 0, WriteFlag::NONE).unwrap();

        // Read result files
        let files = fs.read_dir("/").unwrap();
        assert!(files.iter().any(|f| f.name.starts_with("result/")));
    }

    #[tokio::test]
    async fn test_sqlfs2_connection_string() {
        assert_eq!(extract_database_name("sqlite::memory:"), "memory");
        assert_eq!(extract_database_name("sqlite:/path/to/db.sqlite"), "db");
    }
}
