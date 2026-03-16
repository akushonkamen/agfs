//! FUSE Node - Represents files and directories in the filesystem
//!
//! Maps FUSE inodes to AGFS paths with full operation support.

use ctxfs_sdk::{Client, FileInfo};
use std::sync::Arc;

/// Inode number for root directory
pub const ROOT_INODE: u64 = 1;

/// FUSE node representing a file or directory
#[derive(Debug, Clone)]
pub struct Node {
    /// Inode number (stable across operations)
    pub inode: u64,
    /// File path in AGFS
    pub path: String,
    /// File metadata
    pub info: FileInfo,
    /// Parent inode
    pub parent: u64,
    /// Generation number (for inode reuse detection)
    pub generation: u64,
}

impl Node {
    /// Create root node
    pub fn root() -> Self {
        Self {
            inode: ROOT_INODE,
            path: "/".to_string(),
            info: FileInfo {
                name: "".to_string(),
                size: 0,
                mode: 0o755 | libc::S_IFDIR,
                mod_time: chrono::Utc::now(),
                is_dir: true,
                is_symlink: false,
                meta: ctxfs_sdk::MetaData::default(),
            },
            parent: ROOT_INODE,
            generation: 1,
        }
    }

    /// Create new node
    pub fn new(inode: u64, path: String, info: FileInfo, parent: u64) -> Self {
        Self {
            inode,
            path,
            info,
            parent,
            generation: 1,
        }
    }

    /// Check if this is the root node
    pub fn is_root(&self) -> bool {
        self.inode == ROOT_INODE
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.info.is_dir
    }

    /// Get file mode with type bits
    pub fn mode(&self) -> u32 {
        let mut mode = self.info.mode;
        if self.info.is_symlink {
            mode |= libc::S_IFLNK;
        } else if self.info.is_dir {
            mode |= libc::S_IFDIR;
        } else {
            mode |= libc::S_IFREG;
        }
        mode
    }

    /// Get size
    pub fn size(&self) -> u64 {
        self.info.size as u64
    }
}

/// Node cache mapping inodes to nodes
pub struct NodeCache {
    /// Map from inode to node
    nodes: Arc<tokio::sync::RwLock<std::collections::HashMap<u64, Node>>>,
    /// Map from path to inode
    paths: Arc<tokio::sync::RwLock<std::collections::HashMap<String, u64>>>,
    /// Next inode to allocate
    next_inode: Arc<std::sync::atomic::AtomicU64>,
}

impl NodeCache {
    /// Create new node cache
    pub fn new() -> Self {
        let mut nodes = std::collections::HashMap::new();
        let mut paths = std::collections::HashMap::new();

        // Insert root node
        let root = Node::root();
        nodes.insert(ROOT_INODE, root.clone());
        paths.insert("/".to_string(), ROOT_INODE);

        Self {
            nodes: Arc::new(tokio::sync::RwLock::new(nodes)),
            paths: Arc::new(tokio::sync::RwLock::new(paths)),
            next_inode: Arc::new(std::sync::atomic::AtomicU64::new(ROOT_INODE + 1)),
        }
    }

    /// Allocate a new inode number
    fn allocate_inode(&self) -> u64 {
        self.next_inode
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Get or create node for a path
    pub async fn get_or_create(
        &self,
        client: &Client,
        path: &str,
        parent: u64,
    ) -> Result<Node, ctxfs_sdk::AgfsError> {
        // Check if already cached
        {
            let paths_read = self.paths.read().await;
            if let Some(&inode) = paths_read.get(path) {
                let nodes_read = self.nodes.read().await;
                if let Some(node) = nodes_read.get(&inode) {
                    return Ok(node.clone());
                }
            }
        }

        // Fetch from server
        let info = client.stat(path).await?;

        // Create new node
        let inode = self.allocate_inode();
        let node = Node::new(inode, path.to_string(), info, parent);

        // Cache it
        {
            let mut nodes_write = self.nodes.write().await;
            let mut paths_write = self.paths.write().await;
            nodes_write.insert(inode, node.clone());
            paths_write.insert(path.to_string(), inode);
        }

        Ok(node)
    }

    /// Get node by inode
    pub async fn get(&self, inode: u64) -> Option<Node> {
        let nodes_read = self.nodes.read().await;
        nodes_read.get(&inode).cloned()
    }

    /// Get node by path
    pub async fn get_by_path(&self, path: &str) -> Option<Node> {
        let paths_read = self.paths.read().await;
        if let Some(&inode) = paths_read.get(path) {
            drop(paths_read);
            return self.get(inode).await;
        }
        None
    }

    /// Add a node to cache
    pub async fn insert(&self, path: String, info: FileInfo, parent: u64) -> Node {
        // Check if already exists
        {
            let paths_read = self.paths.read().await;
            if let Some(&inode) = paths_read.get(&path) {
                drop(paths_read);
                if let Some(node) = self.get(inode).await {
                    return node;
                }
            }
        }

        // Create new node
        let inode = self.allocate_inode();
        let node = Node::new(inode, path.clone(), info, parent);

        // Cache it
        {
            let mut nodes_write = self.nodes.write().await;
            let mut paths_write = self.paths.write().await;
            nodes_write.insert(inode, node.clone());
            paths_write.insert(path, inode);
        }

        node
    }

    /// Remove node from cache
    pub async fn remove(&self, path: &str) {
        let mut paths_write = self.paths.write().await;
        if let Some(inode) = paths_write.remove(path) {
            drop(paths_write);
            let mut nodes_write = self.nodes.write().await;
            nodes_write.remove(&inode);
        }
    }

    /// Invalidate all nodes under a prefix
    pub async fn invalidate_prefix(&self, prefix: &str) {
        let mut paths_write = self.paths.write().await;
        let mut nodes_write = self.nodes.write().await;

        let mut to_remove = Vec::new();
        for (path, inode) in paths_write.iter() {
            if path.starts_with(prefix) {
                to_remove.push((path.clone(), *inode));
            }
        }

        for (path, inode) in to_remove {
            paths_write.remove(&path);
            nodes_write.remove(&inode);
        }
    }

    /// Get cache size
    pub async fn len(&self) -> usize {
        self.nodes.read().await.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        self.nodes.read().await.is_empty()
    }

    /// Clear cache
    pub async fn clear(&self) {
        self.nodes.write().await.clear();
        self.paths.write().await.clear();

        // Re-insert root
        let root = Node::root();
        self.nodes.write().await.insert(ROOT_INODE, root);
        self.paths.write().await.insert("/".to_string(), ROOT_INODE);
    }
}

impl Default for NodeCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_root() {
        let root = Node::root();
        assert_eq!(root.inode, ROOT_INODE);
        assert!(root.is_root());
        assert!(root.is_dir());
    }

    #[test]
    fn test_node_creation() {
        let info = FileInfo {
            name: "test.txt".to_string(),
            size: 100,
            mode: 0o644,
            mod_time: chrono::Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: ctxfs_sdk::MetaData::default(),
        };

        let node = Node::new(2, "/test.txt".to_string(), info, 1);
        assert_eq!(node.inode, 2);
        assert_eq!(node.path, "/test.txt");
        assert!(!node.is_dir());
        assert!(!node.is_root());
    }

    #[test]
    fn test_node_mode() {
        let info = FileInfo {
            name: "test.txt".to_string(),
            size: 100,
            mode: 0o644,
            mod_time: chrono::Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: ctxfs_sdk::MetaData::default(),
        };

        let node = Node::new(2, "/test.txt".to_string(), info, 1);
        assert_eq!(node.mode() & libc::S_IFREG, libc::S_IFREG);
        assert_eq!(node.mode() & 0o777, 0o644);
    }

    #[tokio::test]
    async fn test_node_cache() {
        let cache = NodeCache::new();

        // Root should exist
        let root = cache.get(ROOT_INODE).await;
        assert!(root.is_some());
        assert!(root.unwrap().is_root());

        // Get by path
        let root_by_path = cache.get_by_path("/").await;
        assert!(root_by_path.is_some());
    }
}
