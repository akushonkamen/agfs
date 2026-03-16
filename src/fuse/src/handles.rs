//! File Handle Management for FUSE
//!
//! Manages open file handles for FUSE operations.
//! Since AGFS is stateless on the client side, handles are just
//! local identifiers for tracking open files.

use ctxfs_sdk::Client;
use dashmap::DashMap;
use std::sync::Arc;

/// File handle information
#[derive(Debug, Clone)]
pub struct HandleInfo {
    /// File path
    pub path: String,
    /// Open flags
    pub flags: u32,
}

/// Manages open file handles for FUSE operations
pub struct HandleManager {
    /// Map from FUSE handle ID (u64) to HandleInfo
    handles: DashMap<u64, HandleInfo>,
    /// AGFS SDK client
    client: Arc<Client>,
    /// Next local handle ID (for handles without server ID)
    next_handle_id: std::sync::atomic::AtomicU64,
}

impl HandleManager {
    /// Create new handle manager
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            handles: DashMap::new(),
            client,
            next_handle_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Allocate a new local handle ID
    fn allocate_handle_id(&self) -> u64 {
        self.next_handle_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Open a handle for the given path
    pub async fn open_remote(&self, path: &str, flags: u32) -> Result<u64, ctxfs_sdk::AgfsError> {
        // Just allocate a local handle ID - AGFS is stateless
        let local_id = self.allocate_handle_id();
        let info = HandleInfo {
            path: path.to_string(),
            flags,
        };
        self.handles.insert(local_id, info);
        Ok(local_id)
    }

    /// Open a stream handle for queuefs operations
    pub async fn open_stream(
        &self,
        path: &str,
        flags: u32,
    ) -> Result<u64, ctxfs_sdk::AgfsError> {
        self.open_remote(path, flags).await
    }

    /// Open a local handle (fallback)
    pub fn open_local(&self, path: &str, flags: u32) -> Result<u64, ctxfs_sdk::AgfsError> {
        let local_id = self.allocate_handle_id();
        let info = HandleInfo {
            path: path.to_string(),
            flags,
        };
        self.handles.insert(local_id, info);
        Ok(local_id)
    }

    /// Close a handle
    pub async fn close(&self, handle_id: u64) -> Result<(), ctxfs_sdk::AgfsError> {
        self.handles.remove(&handle_id);
        Ok(())
    }

    /// Close all open handles
    pub async fn close_all(&self) -> Result<(), ctxfs_sdk::AgfsError> {
        self.handles.clear();
        Ok(())
    }

    /// Get handle info by ID
    pub fn get(&self, handle_id: u64) -> Option<HandleInfo> {
        self.handles.get(&handle_id).map(|entry| entry.value().clone())
    }

    /// Read from a file (using the path from handle info)
    pub async fn read_remote(
        &self,
        handle_id: u64,
        offset: i64,
        size: i64,
    ) -> Result<Vec<u8>, ctxfs_sdk::AgfsError> {
        let info = self
            .get(handle_id)
            .ok_or_else(|| ctxfs_sdk::AgfsError::invalid_argument("invalid handle"))?;

        // Use SDK's read method with offset support
        self.client.read(&info.path, offset, size).await
    }

    /// Write to a file (using the path from handle info)
    pub async fn write_remote(
        &self,
        handle_id: u64,
        data: &[u8],
        offset: i64,
    ) -> Result<i64, ctxfs_sdk::AgfsError> {
        let info = self
            .get(handle_id)
            .ok_or_else(|| ctxfs_sdk::AgfsError::invalid_argument("invalid handle"))?;

        // Use SDK's write_with_flags method
        self.client.write_with_flags(&info.path, data, offset, ctxfs_sdk::WriteFlag::empty())
            .await?;
        Ok(data.len() as i64)
    }

    /// Get current count of open handles
    pub fn len(&self) -> usize {
        self.handles.len()
    }

    /// Check if any handles are open
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_info_creation() {
        let info = HandleInfo {
            path: "/test.txt".to_string(),
            flags: 0o666,
        };
        assert_eq!(info.path, "/test.txt");
        assert_eq!(info.flags, 0o666);
    }
}
