//! SqlFS - SQL Database File System
//!
//! Stores files as BLOBs in a SQL database (SQLite, MySQL, PostgreSQL).

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use sqlx::{MySqlPool, PgPool, Row, SqlitePool};
use std::pin::Pin;
use std::sync::Arc;

/// Database type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbType {
    SQLite,
    MySQL,
    PostgreSQL,
}

/// Database pool enum that can hold any of the supported database pools
#[derive(Clone)]
pub enum DbPool {
    SQLite(Arc<SqlitePool>),
    MySQL(Arc<MySqlPool>),
    PostgreSQL(Arc<PgPool>),
}

/// SQL file system with multi-database backend support
///
/// Supports SQLite, MySQL, and PostgreSQL through connection pooling.
#[derive(Debug, Clone)]
pub struct SqlFS {
    pool: DbPool,
    db_type: DbType,
    table_prefix: String,
}

impl SqlFS {
    /// Create a new SqlFS instance from connection string
    ///
    /// Connection string format:
    /// - SQLite: `sqlite:path/to/database.db` or `sqlite::memory:`
    /// - MySQL: `mysql://user:password@host/database`
    /// - PostgreSQL: `postgresql://user:password@host/database` or `postgres://...`
    pub async fn new(connection_string: &str) -> Result<Self, AgfsError> {
        Self::with_table_prefix(connection_string, "agfs_files").await
    }

    /// Create a new SqlFS instance with custom table prefix
    pub async fn with_table_prefix(connection_string: &str, table_prefix: &str) -> Result<Self, AgfsError> {
        let (pool, db_type) = if connection_string.starts_with("postgres:") || connection_string.starts_with("postgresql:") {
            // PostgreSQL connection
            let pg_pool = PgPool::connect(connection_string)
                .await
                .map_err(|e| AgfsError::internal(format!("Failed to connect to PostgreSQL: {}", e)))?;
            (DbPool::PostgreSQL(Arc::new(pg_pool)), DbType::PostgreSQL)
        } else if connection_string.starts_with("mysql:") || connection_string.starts_with("mariadb:") {
            // MySQL connection
            let mysql_pool = MySqlPool::connect(connection_string)
                .await
                .map_err(|e| AgfsError::internal(format!("Failed to connect to MySQL: {}", e)))?;
            (DbPool::MySQL(Arc::new(mysql_pool)), DbType::MySQL)
        } else {
            // SQLite connection (default)
            let conn_str = if connection_string.starts_with("sqlite:") {
                connection_string.to_string()
            } else if connection_string.contains(":memory:") {
                format!("sqlite:{}", connection_string)
            } else {
                format!("sqlite:{}", connection_string)
            };
            let sqlite_pool = SqlitePool::connect(&conn_str)
                .await
                .map_err(|e| AgfsError::internal(format!("Failed to connect to SQLite: {}", e)))?;
            (DbPool::SQLite(Arc::new(sqlite_pool)), DbType::SQLite)
        };

        let fs = Self {
            pool,
            db_type,
            table_prefix: table_prefix.to_string(),
        };

        // Initialize schema
        fs.init_schema().await?;

        Ok(fs)
    }

    /// Get the database type
    pub fn db_type(&self) -> DbType {
        self.db_type
    }

    /// Get the files table name
    fn files_table(&self) -> String {
        format!("{}_files", self.table_prefix)
    }

    /// Get the directories table name
    fn dirs_table(&self) -> String {
        format!("{}_dirs", self.table_prefix)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<(), AgfsError> {
        let files_table = self.files_table();
        let dirs_table = self.dirs_table();

        match &self.pool {
            DbPool::PostgreSQL(pool) => {
                // PostgreSQL syntax
                sqlx::query(&format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        path TEXT PRIMARY KEY,
                        data BYTEA,
                        mode BIGINT,
                        mod_time TEXT,
                        size BIGINT
                    )",
                    files_table
                ))
                .execute(&**pool)
                .await
                .map_err(|e| AgfsError::internal(format!("Failed to create files table: {}", e)))?;

                sqlx::query(&format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        path TEXT PRIMARY KEY,
                        mode BIGINT,
                        mod_time TEXT
                    )",
                    dirs_table
                ))
                .execute(&**pool)
                .await
                .map_err(|e| AgfsError::internal(format!("Failed to create dirs table: {}", e)))?;
            }
            DbPool::MySQL(pool) => {
                // MySQL syntax
                sqlx::query(&format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        path VARCHAR(512) PRIMARY KEY,
                        data LONGBLOB,
                        mode BIGINT,
                        mod_time VARCHAR(64),
                        size BIGINT
                    )",
                    files_table
                ))
                .execute(&**pool)
                .await
                .map_err(|e| AgfsError::internal(format!("Failed to create files table: {}", e)))?;

                sqlx::query(&format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        path VARCHAR(512) PRIMARY KEY,
                        mode BIGINT,
                        mod_time VARCHAR(64)
                    )",
                    dirs_table
                ))
                .execute(&**pool)
                .await
                .map_err(|e| AgfsError::internal(format!("Failed to create dirs table: {}", e)))?;
            }
            DbPool::SQLite(pool) => {
                // SQLite syntax
                sqlx::query(&format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        path TEXT PRIMARY KEY,
                        data BLOB,
                        mode INTEGER,
                        mod_time TEXT,
                        size INTEGER
                    )",
                    files_table
                ))
                .execute(&**pool)
                .await
                .map_err(|e| AgfsError::internal(format!("Failed to create files table: {}", e)))?;

                sqlx::query(&format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        path TEXT PRIMARY KEY,
                        mode INTEGER,
                        mod_time TEXT
                    )",
                    dirs_table
                ))
                .execute(&**pool)
                .await
                .map_err(|e| AgfsError::internal(format!("Failed to create dirs table: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Check if a file exists
    async fn file_exists(&self, path: &str) -> Result<bool, AgfsError> {
        let files_table = self.files_table();
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {} WHERE path = $1", files_table))
            .bind(path)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;
        Ok(count > 0)
    }

    /// Check if a directory exists
    async fn dir_exists(&self, path: &str) -> Result<bool, AgfsError> {
        let dirs_table = self.dirs_table();
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {} WHERE path = $1", dirs_table))
            .bind(path)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;
        Ok(count > 0)
    }

    /// Helper to run async code in sync context without runtime nesting
    fn run_in_blocking<F, R>(&self, f: F) -> Result<R, AgfsError>
    where
        F: FnOnce(DbPool, String) -> Pin<Box<dyn futures::Future<Output = Result<R, AgfsError>> + Send>> + Send + 'static,
        R: Send + 'static,
    {
        let pool = self.pool.clone();
        let table_prefix = self.table_prefix.clone();

        // Use std::thread::spawn instead of tokio::task::spawn_blocking
        // since we need to block on the result and we can't await
        let handle = std::thread::spawn(move || {
            // Create a new runtime within the thread
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| AgfsError::internal(format!("failed to create runtime: {}", e)))?;
            rt.block_on(f(pool, table_prefix))
        });

        handle.join().map_err(|e| AgfsError::internal(format!("thread join failed: {:?}", e)))?
    }
}

impl FileSystem for SqlFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        let path = path.to_string();

        self.run_in_blocking(move |pool, table_prefix| Box::pin(async move {
            let files_table = format!("{}_files", table_prefix);

            match &pool {
                DbPool::PostgreSQL(p) => {
                    // Check if file exists
                    let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {} WHERE path = $1", files_table))
                        .bind(&path)
                        .fetch_one(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;

                    if count > 0 {
                        return Err(AgfsError::already_exists(&path));
                    }

                    let now = Utc::now().to_rfc3339();

                    sqlx::query(&format!(
                        "INSERT INTO {} (path, data, mode, mod_time, size) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (path) DO NOTHING",
                        files_table
                    ))
                    .bind(&path)
                    .bind(Vec::<u8>::new())
                    .bind(0i64)
                    .bind(&now)
                    .bind(0i64)
                    .execute(&**p)
                    .await
                    .map_err(|e| AgfsError::internal(format!("Insert failed: {}", e)))?;
                }
                DbPool::MySQL(p) => {
                    // Check if file exists
                    let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {} WHERE path = ?", files_table))
                        .bind(&path)
                        .fetch_one(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;

                    if count > 0 {
                        return Err(AgfsError::already_exists(&path));
                    }

                    let now = Utc::now().to_rfc3339();

                    sqlx::query(&format!(
                        "INSERT INTO {} (path, data, mode, mod_time, size) VALUES (?, ?, ?, ?, ?)",
                        files_table
                    ))
                    .bind(&path)
                    .bind(Vec::<u8>::new())
                    .bind(0i64)
                    .bind(&now)
                    .bind(0i64)
                    .execute(&**p)
                    .await
                    .map_err(|e| AgfsError::internal(format!("Insert failed: {}", e)))?;
                }
                DbPool::SQLite(p) => {
                    // Check if file exists
                    let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {} WHERE path = $1", files_table))
                        .bind(&path)
                        .fetch_one(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;

                    if count > 0 {
                        return Err(AgfsError::already_exists(&path));
                    }

                    let now = Utc::now().to_rfc3339();

                    sqlx::query(&format!(
                        "INSERT INTO {} (path, data, mode, mod_time, size) VALUES ($1, $2, $3, $4, $5)",
                        files_table
                    ))
                    .bind(&path)
                    .bind(Vec::<u8>::new())
                    .bind(0i64)
                    .bind(&now)
                    .bind(0i64)
                    .execute(&**p)
                    .await
                    .map_err(|e| AgfsError::internal(format!("Insert failed: {}", e)))?;
                }
            }

            Ok(())
        }))
    }

    fn mkdir(&self, path: &str, perm: u32) -> Result<(), AgfsError> {
        let path = path.to_string();
        let perm = perm as i64;

        self.run_in_blocking(move |pool, table_prefix| Box::pin(async move {
            let dirs_table = format!("{}_dirs", table_prefix);
            let now = Utc::now().to_rfc3339();

            match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&format!(
                        "INSERT INTO {} (path, mode, mod_time) VALUES ($1, $2, $3) ON CONFLICT (path) DO NOTHING",
                        dirs_table
                    ))
                    .bind(&path)
                    .bind(perm)
                    .bind(&now)
                    .execute(&**p)
                    .await
                    .map_err(|e| AgfsError::internal(format!("Insert dir failed: {}", e)))?;
                }
                DbPool::MySQL(p) => {
                    sqlx::query(&format!(
                        "INSERT IGNORE INTO {} (path, mode, mod_time) VALUES (?, ?, ?)",
                        dirs_table
                    ))
                    .bind(&path)
                    .bind(perm)
                    .bind(&now)
                    .execute(&**p)
                    .await
                    .map_err(|e| AgfsError::internal(format!("Insert dir failed: {}", e)))?;
                }
                DbPool::SQLite(p) => {
                    sqlx::query(&format!(
                        "INSERT INTO {} (path, mode, mod_time) VALUES ($1, $2, $3) ON CONFLICT (path) DO NOTHING",
                        dirs_table
                    ))
                    .bind(&path)
                    .bind(perm)
                    .bind(&now)
                    .execute(&**p)
                    .await
                    .map_err(|e| AgfsError::internal(format!("Insert dir failed: {}", e)))?;
                }
            }

            Ok(())
        }))
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let path = path.to_string();

        self.run_in_blocking(move |pool, table_prefix| Box::pin(async move {
            let files_table = format!("{}_files", table_prefix);
            let dirs_table = format!("{}_dirs", table_prefix);

            // Try to remove as file first
            let result = match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&format!("DELETE FROM {} WHERE path = $1", files_table))
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
                DbPool::MySQL(p) => {
                    sqlx::query(&format!("DELETE FROM {} WHERE path = ?", files_table))
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
                DbPool::SQLite(p) => {
                    sqlx::query(&format!("DELETE FROM {} WHERE path = $1", files_table))
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
            };

            if let Ok(rows) = result {
                if rows.rows_affected() > 0 {
                    return Ok(());
                }
            }

            // Try to remove as directory
            let result = match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&format!("DELETE FROM {} WHERE path = $1", dirs_table))
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
                DbPool::MySQL(p) => {
                    sqlx::query(&format!("DELETE FROM {} WHERE path = ?", dirs_table))
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
                DbPool::SQLite(p) => {
                    sqlx::query(&format!("DELETE FROM {} WHERE path = $1", dirs_table))
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
            };

            let result = result.map_err(|e| AgfsError::internal(format!("Delete dir failed: {}", e)))?;

            if result.rows_affected() > 0 {
                Ok(())
            } else {
                Err(AgfsError::not_found(&path))
            }
        }))
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        let path = path.to_string();

        self.run_in_blocking(move |pool, table_prefix| Box::pin(async move {
            let files_table = format!("{}_files", table_prefix);
            let dirs_table = format!("{}_dirs", table_prefix);

            // Use LIKE pattern matching for MySQL/PostgreSQL, GLOB for SQLite
            let pattern = format!("{}%", path.trim_end_matches('/'));

            match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&format!("DELETE FROM {} WHERE path LIKE $1", files_table))
                        .bind(&pattern)
                        .execute(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Delete files failed: {}", e)))?;

                    sqlx::query(&format!("DELETE FROM {} WHERE path LIKE $1", dirs_table))
                        .bind(&pattern)
                        .execute(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Delete dirs failed: {}", e)))?;
                }
                DbPool::MySQL(p) => {
                    sqlx::query(&format!("DELETE FROM {} WHERE path LIKE ?", files_table))
                        .bind(&pattern)
                        .execute(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Delete files failed: {}", e)))?;

                    sqlx::query(&format!("DELETE FROM {} WHERE path LIKE ?", dirs_table))
                        .bind(&pattern)
                        .execute(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Delete dirs failed: {}", e)))?;
                }
                DbPool::SQLite(p) => {
                    let glob_pattern = format!("{}/*", path.trim_end_matches('/'));
                    sqlx::query(&format!("DELETE FROM {} WHERE path GLOB $1", files_table))
                        .bind(&glob_pattern)
                        .execute(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Delete files failed: {}", e)))?;

                    sqlx::query(&format!("DELETE FROM {} WHERE path GLOB $1", dirs_table))
                        .bind(&glob_pattern)
                        .execute(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Delete dirs failed: {}", e)))?;
                }
            }

            Ok(())
        }))
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let path = path.to_string();

        self.run_in_blocking(move |pool, table_prefix| Box::pin(async move {
            let files_table = format!("{}_files", table_prefix);

            let data: Vec<u8> = if size < 0 && offset <= 0 {
                // Read entire file
                match &pool {
                    DbPool::PostgreSQL(p) => {
                        sqlx::query_scalar(&format!("SELECT data FROM {} WHERE path = $1", files_table))
                            .bind(&path)
                            .fetch_one(&**p)
                            .await
                            .map_err(|e| AgfsError::internal(format!("Read failed: {}", e)))?
                    }
                    DbPool::MySQL(p) => {
                        sqlx::query_scalar(&format!("SELECT data FROM {} WHERE path = ?", files_table))
                            .bind(&path)
                            .fetch_one(&**p)
                            .await
                            .map_err(|e| AgfsError::internal(format!("Read failed: {}", e)))?
                    }
                    DbPool::SQLite(p) => {
                        sqlx::query_scalar(&format!("SELECT data FROM {} WHERE path = $1", files_table))
                            .bind(&path)
                            .fetch_one(&**p)
                            .await
                            .map_err(|e| AgfsError::internal(format!("Read failed: {}", e)))?
                    }
                }
            } else {
                // Use SUBSTR for partial reads (works across all DBs)
                let substr_offset = offset.max(0) + 1;
                let substr_len = if size < 0 { 1000000000 } else { size };
                match &pool {
                    DbPool::PostgreSQL(p) => {
                        sqlx::query_scalar(&format!("SELECT SUBSTRING(data, $2, $3) FROM {} WHERE path = $1", files_table))
                            .bind(&path)
                            .bind(substr_offset)
                            .bind(substr_len)
                            .fetch_one(&**p)
                            .await
                            .map_err(|e| AgfsError::internal(format!("Read failed: {}", e)))?
                    }
                    DbPool::MySQL(p) => {
                        sqlx::query_scalar(&format!("SELECT SUBSTRING(data, ?, ?) FROM {} WHERE path = ?", files_table))
                            .bind(&path)
                            .bind(substr_offset)
                            .bind(substr_len)
                            .fetch_one(&**p)
                            .await
                            .map_err(|e| AgfsError::internal(format!("Read failed: {}", e)))?
                    }
                    DbPool::SQLite(p) => {
                        sqlx::query_scalar(&format!("SELECT SUBSTR(data, $2, $3) FROM {} WHERE path = $1", files_table))
                            .bind(&path)
                            .bind(substr_offset)
                            .bind(substr_len)
                            .fetch_one(&**p)
                            .await
                            .map_err(|e| AgfsError::internal(format!("Read failed: {}", e)))?
                    }
                }
            };

            Ok(data)
        }))
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, _flags: WriteFlag) -> Result<i64, AgfsError> {
        let path = path.to_string();
        let data = data.to_vec();
        let now = Utc::now().to_rfc3339();
        let len = data.len() as i64;

        self.run_in_blocking(move |pool, table_prefix| Box::pin(async move {
            let files_table = format!("{}_files", table_prefix);

            let rows_affected = match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&format!("UPDATE {} SET data = $1, mod_time = $2, size = $3 WHERE path = $4", files_table))
                        .bind(&data)
                        .bind(&now)
                        .bind(len)
                        .bind(&path)
                        .execute(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Update failed: {}", e)))?
                        .rows_affected()
                }
                DbPool::MySQL(p) => {
                    sqlx::query(&format!("UPDATE {} SET data = ?, mod_time = ?, size = ? WHERE path = ?", files_table))
                        .bind(&data)
                        .bind(&now)
                        .bind(len)
                        .bind(&path)
                        .execute(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Update failed: {}", e)))?
                        .rows_affected()
                }
                DbPool::SQLite(p) => {
                    sqlx::query(&format!("UPDATE {} SET data = $1, mod_time = $2, size = $3 WHERE path = $4", files_table))
                        .bind(&data)
                        .bind(&now)
                        .bind(len)
                        .bind(&path)
                        .execute(&**p)
                        .await
                        .map_err(|e| AgfsError::internal(format!("Update failed: {}", e)))?
                        .rows_affected()
                }
            };

            if rows_affected == 0 {
                return Err(AgfsError::not_found(&path));
            }

            Ok(len)
        }))
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        let path = path.to_string();

        self.run_in_blocking(move |pool, table_prefix| Box::pin(async move {
            let files_table = format!("{}_files", table_prefix);
            let dirs_table = format!("{}_dirs", table_prefix);
            let mut files = Vec::new();

            // Build query based on database type
            let (file_query, dir_query) = match &pool {
                DbPool::PostgreSQL(_) | DbPool::MySQL(_) => {
                    // Use LIKE with wildcard
                    let pattern = format!("{}/%", path.trim_end_matches('/'));
                    let fq = format!("SELECT path, mode, mod_time, size FROM {} WHERE path LIKE $1", files_table);
                    let dq = format!("SELECT path, mode, mod_time FROM {} WHERE path LIKE $1", dirs_table);
                    (fq, dq)
                }
                DbPool::SQLite(_) => {
                    // Use GLOB
                    let glob_pattern = format!("{}/*", path.trim_end_matches('/'));
                    let fq = format!("SELECT path, mode, mod_time, size FROM {} WHERE path GLOB $1", files_table);
                    let dq = format!("SELECT path, mode, mod_time FROM {} WHERE path GLOB $1", dirs_table);
                    (fq, dq)
                }
            };

            let pattern = match &pool {
                DbPool::PostgreSQL(_) | DbPool::MySQL(_) => format!("{}/%", path.trim_end_matches('/')),
                DbPool::SQLite(_) => format!("{}/*", path.trim_end_matches('/')),
            };

            // Query files in this directory
            macro_rules! query_files {
                ($pool:expr, $query:expr, $pattern:expr) => {{
                    sqlx::query(&$query)
                        .bind(&$pattern)
                        .fetch_all($pool)
                        .await
                }}
            }

            let file_rows = match &pool {
                DbPool::PostgreSQL(p) => query_files!(&***p, file_query, pattern).await,
                DbPool::MySQL(p) => {
                    // MySQL uses ? placeholder
                    let mq = format!("SELECT path, mode, mod_time, size FROM {} WHERE path LIKE ?", files_table);
                    sqlx::query(&mq).bind(&pattern).fetch_all(&**p).await
                }
                DbPool::SQLite(p) => query_files!(&***p, file_query, pattern).await,
            };

            let file_rows = file_rows.map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;

            for row in file_rows {
                let full_path: String = row.get("path");
                let name = full_path.trim_start_matches(&path).trim_start_matches('/').to_string();

                // Skip if contains another / (subdirectory)
                if name.contains('/') {
                    continue;
                }

                files.push(FileInfo {
                    name,
                    size: row.get("size"),
                    mode: row.get::<i64, _>("mode") as u32,
                    mod_time: chrono::DateTime::parse_from_rfc3339(row.get("mod_time"))
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    is_dir: false,
                    is_symlink: false,
                    meta: MetaData::default(),
                });
            }

            // Query directories in this directory
            let dir_rows = match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&dir_query).bind(&pattern).fetch_all(&**p).await
                }
                DbPool::MySQL(p) => {
                    let mq = format!("SELECT path, mode, mod_time FROM {} WHERE path LIKE ?", dirs_table);
                    sqlx::query(&mq).bind(&pattern).fetch_all(&**p).await
                }
                DbPool::SQLite(p) => {
                    sqlx::query(&dir_query).bind(&pattern).fetch_all(&**p).await
                }
            };

            let dir_rows = dir_rows.map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;

            for row in dir_rows {
                let full_path: String = row.get("path");
                let name = full_path.trim_start_matches(&path).trim_start_matches('/').to_string();

                if name.contains('/') {
                    continue;
                }

                files.push(FileInfo {
                    name,
                    size: 0,
                    mode: row.get::<i64, _>("mode") as u32,
                    mod_time: chrono::DateTime::parse_from_rfc3339(row.get("mod_time"))
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    is_dir: true,
                    is_symlink: false,
                    meta: MetaData::default(),
                });
            }

            Ok(files)
        }))
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        let path = path.to_string();

        self.run_in_blocking(move |pool, table_prefix| Box::pin(async move {
            if path == "/" || path.is_empty() {
                return Ok(FileInfo {
                    name: String::new(),
                    size: 0,
                    mode: 0o555,
                    mod_time: Utc::now(),
                    is_dir: true,
                    is_symlink: false,
                    meta: MetaData::default(),
                });
            }

            let files_table = format!("{}_files", table_prefix);
            let dirs_table = format!("{}_dirs", table_prefix);

            // Try as file first
            let file_result = match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&format!("SELECT path, mode, mod_time, size FROM {} WHERE path = $1", files_table))
                        .bind(&path)
                        .fetch_optional(&**p)
                        .await
                }
                DbPool::MySQL(p) => {
                    sqlx::query(&format!("SELECT path, mode, mod_time, size FROM {} WHERE path = ?", files_table))
                        .bind(&path)
                        .fetch_optional(&**p)
                        .await
                }
                DbPool::SQLite(p) => {
                    sqlx::query(&format!("SELECT path, mode, mod_time, size FROM {} WHERE path = $1", files_table))
                        .bind(&path)
                        .fetch_optional(&**p)
                        .await
                }
            };

            if let Ok(Some(row)) = file_result {
                return Ok(FileInfo {
                    name: path.trim_start_matches('/').to_string(),
                    size: row.get("size"),
                    mode: row.get::<i64, _>("mode") as u32,
                    mod_time: chrono::DateTime::parse_from_rfc3339(row.get("mod_time"))
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    is_dir: false,
                    is_symlink: false,
                    meta: MetaData::default(),
                });
            }

            // Try as directory
            let dir_result = match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&format!("SELECT path, mode, mod_time FROM {} WHERE path = $1", dirs_table))
                        .bind(&path)
                        .fetch_optional(&**p)
                        .await
                }
                DbPool::MySQL(p) => {
                    sqlx::query(&format!("SELECT path, mode, mod_time FROM {} WHERE path = ?", dirs_table))
                        .bind(&path)
                        .fetch_optional(&**p)
                        .await
                }
                DbPool::SQLite(p) => {
                    sqlx::query(&format!("SELECT path, mode, mod_time FROM {} WHERE path = $1", dirs_table))
                        .bind(&path)
                        .fetch_optional(&**p)
                        .await
                }
            };

            if let Ok(Some(row)) = dir_result {
                return Ok(FileInfo {
                    name: path.trim_start_matches('/').to_string(),
                    size: 0,
                    mode: row.get::<i64, _>("mode") as u32,
                    mod_time: chrono::DateTime::parse_from_rfc3339(row.get("mod_time"))
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    is_dir: true,
                    is_symlink: false,
                    meta: MetaData::default(),
                });
            }

            Err(AgfsError::not_found(&path))
        }))
    }

    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError> {
        let old_path = old_path.to_string();
        let new_path = new_path.to_string();

        self.run_in_blocking(move |pool, table_prefix| Box::pin(async move {
            let files_table = format!("{}_files", table_prefix);
            let dirs_table = format!("{}_dirs", table_prefix);

            // Try file rename first
            let result = match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&format!("UPDATE {} SET path = $1 WHERE path = $2", files_table))
                        .bind(&new_path)
                        .bind(&old_path)
                        .execute(&**p)
                        .await
                }
                DbPool::MySQL(p) => {
                    sqlx::query(&format!("UPDATE {} SET path = ? WHERE path = ?", files_table))
                        .bind(&new_path)
                        .bind(&old_path)
                        .execute(&**p)
                        .await
                }
                DbPool::SQLite(p) => {
                    sqlx::query(&format!("UPDATE {} SET path = $1 WHERE path = $2", files_table))
                        .bind(&new_path)
                        .bind(&old_path)
                        .execute(&**p)
                        .await
                }
            };

            if let Ok(rows) = result {
                if rows.rows_affected() > 0 {
                    return Ok(());
                }
            }

            // Try directory rename
            let result = match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&format!("UPDATE {} SET path = $1 WHERE path = $2", dirs_table))
                        .bind(&new_path)
                        .bind(&old_path)
                        .execute(&**p)
                        .await
                }
                DbPool::MySQL(p) => {
                    sqlx::query(&format!("UPDATE {} SET path = ? WHERE path = ?", dirs_table))
                        .bind(&new_path)
                        .bind(&old_path)
                        .execute(&**p)
                        .await
                }
                DbPool::SQLite(p) => {
                    sqlx::query(&format!("UPDATE {} SET path = $1 WHERE path = $2", dirs_table))
                        .bind(&new_path)
                        .bind(&old_path)
                        .execute(&**p)
                        .await
                }
            };

            let result = result.map_err(|e| AgfsError::internal(format!("Rename dir failed: {}", e)))?;

            if result.rows_affected() > 0 {
                Ok(())
            } else {
                Err(AgfsError::not_found(&old_path))
            }
        }))
    }

    fn chmod(&self, path: &str, mode: u32) -> Result<(), AgfsError> {
        let path = path.to_string();
        let mode = mode as i64;

        self.run_in_blocking(move |pool, table_prefix| Box::pin(async move {
            let files_table = format!("{}_files", table_prefix);
            let dirs_table = format!("{}_dirs", table_prefix);

            // Try file chmod first
            let result = match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&format!("UPDATE {} SET mode = $1 WHERE path = $2", files_table))
                        .bind(mode)
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
                DbPool::MySQL(p) => {
                    sqlx::query(&format!("UPDATE {} SET mode = ? WHERE path = ?", files_table))
                        .bind(mode)
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
                DbPool::SQLite(p) => {
                    sqlx::query(&format!("UPDATE {} SET mode = $1 WHERE path = $2", files_table))
                        .bind(mode)
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
            };

            if let Ok(rows) = result {
                if rows.rows_affected() > 0 {
                    return Ok(());
                }
            }

            // Try directory chmod
            let result = match &pool {
                DbPool::PostgreSQL(p) => {
                    sqlx::query(&format!("UPDATE {} SET mode = $1 WHERE path = $2", dirs_table))
                        .bind(mode)
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
                DbPool::MySQL(p) => {
                    sqlx::query(&format!("UPDATE {} SET mode = ? WHERE path = ?", dirs_table))
                        .bind(mode)
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
                DbPool::SQLite(p) => {
                    sqlx::query(&format!("UPDATE {} SET mode = $1 WHERE path = $2", dirs_table))
                        .bind(mode)
                        .bind(&path)
                        .execute(&**p)
                        .await
                }
            };

            let result = result.map_err(|e| AgfsError::internal(format!("Chmod failed: {}", e)))?;

            if result.rows_affected() > 0 {
                Ok(())
            } else {
                Err(AgfsError::not_found(&path))
            }
        }))
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
    async fn test_sqlfs_sqlite() {
        let fs = SqlFS::new("sqlite::memory:").await.unwrap();

        // Test create and write
        fs.create("/test.txt").unwrap();
        fs.write("/test.txt", b"hello from sqlfs", 0, WriteFlag::NONE).unwrap();

        // Test read
        let data = fs.read("/test.txt", 0, -1).unwrap();
        assert_eq!(data, b"hello from sqlfs");

        // Test stat
        let info = fs.stat("/test.txt").unwrap();
        assert_eq!(info.size, 16);
        assert!(!info.is_dir);

        // Test mkdir
        fs.mkdir("/testdir", 0o755).unwrap();
        let dir_info = fs.stat("/testdir").unwrap();
        assert!(dir_info.is_dir);

        // Test read_dir
        let files = fs.read_dir("/").unwrap();
        assert!(files.iter().any(|f| f.name == "test.txt"));
        assert!(files.iter().any(|f| f.name == "testdir"));

        // Test remove
        fs.remove("/test.txt").unwrap();
        assert!(fs.read("/test.txt", 0, -1).is_err());
    }
}
