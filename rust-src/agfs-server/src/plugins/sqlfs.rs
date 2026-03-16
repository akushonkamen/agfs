//! SqlFS - SQL Database File System
//!
//! Stores files as BLOBs in a SQL database (SQLite, MySQL, PostgreSQL).

use agfs_sdk::{AgfsError, FileInfo, FileSystem, MetaData, WriteFlag};
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

/// SQL file system with SQLite backend
///
/// For MySQL/PostgreSQL support, use database-specific connection pooling
/// and execute SQL directly through those clients.
#[derive(Debug, Clone)]
pub struct SqlFS {
    pool: Arc<SqlitePool>,
    table_prefix: String,
}

impl SqlFS {
    /// Create a new SqlFS instance from connection string
    ///
    /// Connection string format:
    /// - SQLite: `sqlite:path/to/database.db` or `sqlite::memory:`
    pub async fn new(connection_string: &str) -> Result<Self, AgfsError> {
        Self::with_table_prefix(connection_string, "agfs_files").await
    }

    /// Create a new SqlFS instance with custom table prefix
    pub async fn with_table_prefix(connection_string: &str, table_prefix: &str) -> Result<Self, AgfsError> {
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

        let fs = Self {
            pool: Arc::new(pool),
            table_prefix: table_prefix.to_string(),
        };

        // Initialize schema
        fs.init_schema().await?;

        Ok(fs)
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

        // Create files table
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
        .execute(&*self.pool)
        .await
        .map_err(|e| AgfsError::internal(format!("Failed to create files table: {}", e)))?;

        // Create directories table
        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS {} (
                path TEXT PRIMARY KEY,
                mode INTEGER,
                mod_time TEXT
            )",
            dirs_table
        ))
        .execute(&*self.pool)
        .await
        .map_err(|e| AgfsError::internal(format!("Failed to create dirs table: {}", e)))?;

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
}

impl FileSystem for SqlFS {
    fn create(&self, path: &str) -> Result<(), AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            if self.file_exists(path).await? {
                return Err(AgfsError::already_exists(path));
            }

            let files_table = self.files_table();
            let now = Utc::now().to_rfc3339();

            sqlx::query(&format!(
                "INSERT INTO {} (path, data, mode, mod_time, size) VALUES ($1, $2, $3, $4, $5)",
                files_table
            ))
            .bind(path)
            .bind(Vec::<u8>::new())
            .bind(0i64)  // mode as i64 for compatibility
            .bind(&now)
            .bind(0i64)
            .execute(&*self.pool)
            .await
            .map_err(|e| AgfsError::internal(format!("Insert failed: {}", e)))?;

            Ok(())
        })
    }

    fn mkdir(&self, path: &str, perm: u32) -> Result<(), AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let dirs_table = self.dirs_table();
            let now = Utc::now().to_rfc3339();

            sqlx::query(&format!(
                "INSERT INTO {} (path, mode, mod_time) VALUES ($1, $2, $3) ON CONFLICT (path) DO NOTHING",
                dirs_table
            ))
            .bind(path)
            .bind(perm as i64)  // u32 to i64
            .bind(&now)
            .execute(&*self.pool)
            .await
            .map_err(|e| AgfsError::internal(format!("Insert dir failed: {}", e)))?;

            Ok(())
        })
    }

    fn remove(&self, path: &str) -> Result<(), AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            // Try to remove as file first
            let files_table = self.files_table();
            let result = sqlx::query(&format!("DELETE FROM {} WHERE path = $1", files_table))
                .bind(path)
                .execute(&*self.pool)
                .await;

            if let Ok(rows) = result {
                if rows.rows_affected() > 0 {
                    return Ok(());
                }
            }

            // Try to remove as directory
            let dirs_table = self.dirs_table();
            let result = sqlx::query(&format!("DELETE FROM {} WHERE path = $1", dirs_table))
                .bind(path)
                .execute(&*self.pool)
                .await
                .map_err(|e| AgfsError::internal(format!("Delete dir failed: {}", e)))?;

            if result.rows_affected() > 0 {
                Ok(())
            } else {
                Err(AgfsError::not_found(path))
            }
        })
    }

    fn remove_all(&self, path: &str) -> Result<(), AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let files_table = self.files_table();
            let dirs_table = self.dirs_table();

            // Use GLOB pattern matching for SQLite
            let glob_pattern = format!("{}/*", path.trim_end_matches('/'));

            sqlx::query(&format!("DELETE FROM {} WHERE path GLOB $1", files_table))
                .bind(&glob_pattern)
                .execute(&*self.pool)
                .await
                .map_err(|e| AgfsError::internal(format!("Delete files failed: {}", e)))?;

            sqlx::query(&format!("DELETE FROM {} WHERE path GLOB $1", dirs_table))
                .bind(&glob_pattern)
                .execute(&*self.pool)
                .await
                .map_err(|e| AgfsError::internal(format!("Delete dirs failed: {}", e)))?;

            Ok(())
        })
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let files_table = self.files_table();

            // Use SUBSTR with 0-index
            let data: Vec<u8> = sqlx::query_scalar(&format!(
                "SELECT SUBSTR(data, $2, $3) FROM {} WHERE path = $1",
                files_table
            ))
            .bind(path)
            .bind(offset.max(0) + 1) // SUBSTR is 1-indexed
            .bind(if size < 0 { -1i64 } else { size })
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| AgfsError::internal(format!("Read failed: {}", e)))?;

            Ok(data)
        })
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, _flags: WriteFlag) -> Result<i64, AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let files_table = self.files_table();
            let now = Utc::now().to_rfc3339();
            let len = data.len() as i64;

            sqlx::query(&format!(
                "UPDATE {} SET data = $1, mod_time = $2, size = $3 WHERE path = $4",
                files_table
            ))
            .bind(data)
            .bind(&now)
            .bind(len)
            .bind(path)
            .execute(&*self.pool)
            .await
            .map_err(|e| AgfsError::internal(format!("Update failed: {}", e)))?;

            Ok(len)
        })
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let files_table = self.files_table();
            let dirs_table = self.dirs_table();
            let mut files = Vec::new();

            // Query files in this directory
            let glob_pattern = format!("{}/%", path.trim_end_matches('/'));
            let file_rows = sqlx::query(&format!(
                "SELECT path, mode, mod_time, size FROM {} WHERE path GLOB $1",
                files_table
            ))
            .bind(&glob_pattern)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;

            for row in file_rows {
                let full_path: String = row.get("path");
                let name = full_path.trim_start_matches(path).trim_start_matches('/').to_string();

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
            let dir_rows = sqlx::query(&format!(
                "SELECT path, mode, mod_time FROM {} WHERE path GLOB $1",
                dirs_table
            ))
            .bind(&glob_pattern)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| AgfsError::internal(format!("Query failed: {}", e)))?;

            for row in dir_rows {
                let full_path: String = row.get("path");
                let name = full_path.trim_start_matches(path).trim_start_matches('/').to_string();

                // Skip if contains another /
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
        })
    }

    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
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

            // Try as file first
            let files_table = self.files_table();
            if let Ok(row) = sqlx::query(&format!(
                "SELECT path, mode, mod_time, size FROM {} WHERE path = $1",
                files_table
            ))
            .bind(path)
            .fetch_one(&*self.pool)
            .await
            {
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
            let dirs_table = self.dirs_table();
            if let Ok(row) = sqlx::query(&format!(
                "SELECT path, mode, mod_time FROM {} WHERE path = $1",
                dirs_table
            ))
            .bind(path)
            .fetch_one(&*self.pool)
            .await
            {
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

            Err(AgfsError::not_found(path))
        })
    }

    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let files_table = self.files_table();
            let dirs_table = self.dirs_table();

            // Try file rename first
            let result = sqlx::query(&format!(
                "UPDATE {} SET path = $1 WHERE path = $2",
                files_table
            ))
            .bind(new_path)
            .bind(old_path)
            .execute(&*self.pool)
            .await;

            if let Ok(rows) = result {
                if rows.rows_affected() > 0 {
                    return Ok(());
                }
            }

            // Try directory rename
            let result = sqlx::query(&format!(
                "UPDATE {} SET path = $1 WHERE path = $2",
                dirs_table
            ))
            .bind(new_path)
            .bind(old_path)
            .execute(&*self.pool)
            .await
            .map_err(|e| AgfsError::internal(format!("Rename dir failed: {}", e)))?;

            if result.rows_affected() > 0 {
                Ok(())
            } else {
                Err(AgfsError::not_found(old_path))
            }
        })
    }

    fn chmod(&self, path: &str, mode: u32) -> Result<(), AgfsError> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AgfsError::internal("no tokio runtime".to_string()))?;

        runtime.block_on(async move {
            let files_table = self.files_table();
            let dirs_table = self.dirs_table();

            // Try file chmod first
            let result = sqlx::query(&format!("UPDATE {} SET mode = $1 WHERE path = $2", files_table))
                .bind(mode as i64)  // u32 to i64
                .bind(path)
                .execute(&*self.pool)
                .await;

            if let Ok(rows) = result {
                if rows.rows_affected() > 0 {
                    return Ok(());
                }
            }

            // Try directory chmod
            let result = sqlx::query(&format!("UPDATE {} SET mode = $1 WHERE path = $2", dirs_table))
                .bind(mode as i64)  // u32 to i64
                .bind(path)
                .execute(&*self.pool)
                .await
                .map_err(|e| AgfsError::internal(format!("Chmod failed: {}", e)))?;

            if result.rows_affected() > 0 {
                Ok(())
            } else {
                Err(AgfsError::not_found(path))
            }
        })
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
