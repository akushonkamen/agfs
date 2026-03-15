//! AGFS common types

// TODO: Remove this allow once full implementation is complete
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub mode: u32,
    pub mod_time: chrono::DateTime<chrono::Utc>,
    pub is_dir: bool,
}

/// Extended metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaData {
    pub file_type: String,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub mod_time: i64,
    pub create_time: i64,
    pub access_time: i64,
}

/// Handle information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleInfo {
    pub id: u64,
    pub path: String,
    pub flags: u32,
}

/// File open flags
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OpenFlag {
    pub read: bool,
    pub write: bool,
    pub append: bool,
    pub create: bool,
    pub exclusive: bool,
    pub truncate: bool,
}

/// Write flags
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WriteFlag {
    pub sync: bool,
    pub append: bool,
}
