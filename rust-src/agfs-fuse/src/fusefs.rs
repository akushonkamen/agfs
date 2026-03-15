//! AGFS FUSE Filesystem Implementation
//!
//! Mounts an AGFS server as a local FUSE filesystem on Linux.

#[cfg(target_os = "linux")]
use agfs_sdk::{Client, FileInfo, WriteFlag};
use chrono::Utc;
use fuse3::raw::prelude::*;
use fuse3::{FileType, Result, SetAttr, Timestamp};
use futures_util::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::cache::{DirectoryCache, MetadataCache};
use crate::handles::HandleManager;
use crate::node::{NodeCache, ROOT_INODE};

/// FUSE filesystem configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub server_url: String,
    pub cache_ttl: std::time::Duration,
    pub debug: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8080/api/v1".to_string(),
            cache_ttl: std::time::Duration::from_secs(30),
            debug: false,
        }
    }
}

/// AGFS FUSE filesystem root
#[derive(Clone)]
pub struct AGFSFS {
    client: Arc<Client>,
    handles: Arc<RwLock<HandleManager>>,
    meta_cache: Arc<RwLock<MetadataCache>>,
    dir_cache: Arc<RwLock<DirectoryCache>>,
    node_cache: Arc<RwLock<NodeCache>>,
    cache_ttl: std::time::Duration,
    debug: bool,
}

impl AGFSFS {
    /// Create a new AGFS FUSE filesystem
    pub fn new(config: Config) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let client = Arc::new(Client::new(&config.server_url)?);
        let handles = Arc::new(RwLock::new(HandleManager::new(client.clone())));

        Ok(Self {
            client,
            handles,
            meta_cache: Arc::new(RwLock::new(MetadataCache::new(config.cache_ttl))),
            dir_cache: Arc::new(RwLock::new(DirectoryCache::new(config.cache_ttl))),
            node_cache: Arc::new(RwLock::new(NodeCache::new())),
            cache_ttl: config.cache_ttl,
            debug: config.debug,
        })
    }

    /// Get the client
    pub fn client(&self) -> &Arc<Client> {
        &self.client
    }

    /// Get the handle manager
    pub fn handles(&self) -> &Arc<RwLock<HandleManager>> {
        &self.handles
    }

    /// Invalidate cache for a path
    pub fn invalidate_cache(&self, path: &str) {
        self.meta_cache.blocking_write().invalidate(path);
        self.dir_cache.blocking_write().invalidate(path);
    }

    /// Invalidate parent directory cache
    pub fn invalidate_parent_cache(&self, path: &str) {
        let parent = get_parent_path(path);
        if !parent.is_empty() {
            self.dir_cache.blocking_write().invalidate(&parent);
        }
    }

    /// Close the filesystem and release resources
    pub async fn close(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        // Close all open handles
        self.handles.write().await.close_all().await?;

        // Clear caches
        self.meta_cache.write().await.clear();
        self.dir_cache.write().await.clear();
        self.node_cache.write().await.clear();

        Ok(())
    }

    /// Get file info with caching
    async fn get_file_info(&self, path: &str) -> std::result::Result<FileInfo, agfs_sdk::AgfsError> {
        // Try cache first
        if let Some(info) = self.meta_cache.read().await.get(path) {
            return Ok(info);
        }

        // Fetch from server
        let info = self.client.stat(path).await?;

        // Cache the result
        self.meta_cache.write().await.insert(path.to_string(), info.clone());

        Ok(info)
    }

    /// Get directory entries with caching
    async fn read_dir_cached(&self, path: &str) -> std::result::Result<Vec<FileInfo>, agfs_sdk::AgfsError> {
        // Try cache first
        if let Some(files) = self.dir_cache.read().await.get(path) {
            return Ok(files);
        }

        // Fetch from server
        let files = self.client.read_dir(path).await?;

        // Cache the result
        self.dir_cache.write().await.insert(path.to_string(), files.clone());

        Ok(files)
    }

    /// Convert FileInfo to FileAttr
    fn file_attr_from_info(&self, info: &FileInfo, ino: u64) -> FileAttr {
        let kind = if info.is_symlink {
            FileType::Symlink
        } else if info.is_dir {
            FileType::Directory
        } else {
            FileType::RegularFile
        };

        let ts = Timestamp {
            sec: info.mod_time.timestamp(),
            nsec: info.mod_time.timestamp_subsec_nanos(),
        };

        #[cfg(target_os = "macos")]
        {
            FileAttr {
                ino,
                size: info.size as u64,
                blocks: (info.size as u64 + 511) / 512,
                atime: ts,
                mtime: ts,
                ctime: ts,
                crtime: ts,
                kind,
                perm: (info.mode & 0o777) as u16,
                nlink: 1,
                uid: unsafe { libc::getuid() } as u32,
                gid: unsafe { libc::getgid() } as u32,
                rdev: 0,
                blksize: 4096,
                flags: 0,
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            FileAttr {
                ino,
                size: info.size as u64,
                blocks: (info.size as u64 + 511) / 512,
                atime: ts,
                mtime: ts,
                ctime: ts,
                kind,
                perm: (info.mode & 0o777) as u16,
                nlink: 1,
                uid: unsafe { libc::getuid() } as u32,
                gid: unsafe { libc::getgid() } as u32,
                rdev: 0,
                blksize: 4096,
            }
        }
    }
}

/// Get parent directory path
fn get_parent_path(path: &str) -> String {
    if path.is_empty() || path == "/" {
        return String::new();
    }

    if let Some(last_slash) = path.rfind('/') {
        if last_slash == 0 {
            return "/".to_string();
        }
        return path[..last_slash].to_string();
    }

    "/".to_string()
}

/// Convert FUSE open flags to WriteFlag
fn convert_open_flags(flags: u32) -> WriteFlag {
    let access_mode = flags & (libc::O_ACCMODE as u32);

    let mut write_flag = WriteFlag::empty();

    match access_mode {
        x if x == (libc::O_RDONLY as u32) => {
            // Read-only - no write flags needed
        }
        x if x == (libc::O_WRONLY as u32) => {
            // Write-only - no specific flag needed
        }
        x if x == (libc::O_RDWR as u32) => {
            // Read-write - no specific flag needed
        }
        _ => {}
    }

    if flags & (libc::O_APPEND as u32) != 0 {
        write_flag |= WriteFlag::APPEND;
    }
    if flags & (libc::O_CREAT as u32) != 0 {
        write_flag |= WriteFlag::CREATE;
    }
    if flags & (libc::O_EXCL as u32) != 0 {
        write_flag |= WriteFlag::EXCLUSIVE;
    }
    if flags & (libc::O_TRUNC as u32) != 0 {
        write_flag |= WriteFlag::TRUNCATE;
    }
    if flags & (libc::O_SYNC as u32) != 0 {
        write_flag |= WriteFlag::SYNC;
    }

    write_flag
}

/// Convert AGFS mode to FUSE file mode
pub fn mode_to_file_mode(mode: u32) -> u32 {
    mode
}

/// Get stable mode with file type bits for FUSE attributes
pub fn get_stable_mode(info: &FileInfo) -> u32 {
    let mut mode = info.mode;

    if info.is_symlink {
        mode |= libc::S_IFLNK;
    } else if info.is_dir {
        mode |= libc::S_IFDIR;
    } else {
        mode |= libc::S_IFREG;
    }

    mode
}

#[cfg(target_os = "linux")]
impl Filesystem for AGFSFS {
    /// Directory entry stream type
    type DirEntryStream<'a>
        = Pin<Box<dyn futures_util::Stream<Item = Result<DirectoryEntry>> + Send + 'a>>
    where
        Self: 'a;

    /// Directory entry plus stream type (not used, default implementation)
    type DirEntryPlusStream<'a>
        = Pin<Box<dyn futures_util::Stream<Item = Result<DirectoryEntryPlus>> + Send + 'a>>
    where
        Self: 'a;
    /// Initialize filesystem
    async fn init(&self, _req: Request) -> Result<ReplyInit> {
        if self.debug {
            eprintln!("[fuse] AGFS FUSE initialized");
        }
        Ok(ReplyInit {
            max_write: std::num::NonZeroU32::new(1024 * 1024).unwrap(),
        })
    }

    /// Destroy filesystem
    async fn destroy(&self, _req: Request) {
        if self.debug {
            eprintln!("[fuse] AGFS FUSE destroyed");
        }
    }

    /// Forget an inode
    async fn forget(&self, _req: Request, _ino: u64, _nlookup: u64) {
        // We manage node lifecycle internally
    }

    /// Get filesystem statistics
    async fn statfs(&self, _req: Request, _ino: u64) -> Result<ReplyStatFs> {
        Ok(ReplyStatFs {
            blocks: 1024 * 1024 * 1024, // 1TB
            bfree: 512 * 1024 * 1024,   // 512GB free
            bavail: 512 * 1024 * 1024,  // 512GB available
            files: 1000000,             // 1M files
            ffree: 500000,              // 500K free inodes
            bsize: 4096,                // 4KB block size
            namelen: 255,               // Max filename length
            frsize: 4096,               // Fragment size
        })
    }

    /// Lookup a directory entry by name
    async fn lookup(&self, _req: Request, parent: u64, name: &std::ffi::OsStr) -> Result<ReplyEntry> {
        let name_str = name.to_string_lossy();
        let path = if parent == ROOT_INODE {
            format!("/{}", name_str)
        } else {
            // Get parent path from node cache
            let node_cache = self.node_cache.read().await;
            if let Some(parent_node) = node_cache.get(parent).await {
                format!("{}{}", parent_node.path, name_str)
            } else {
                return Err(libc::ENOENT.into());
            }
        };

        // Get file info
        let info = self.get_file_info(&path).await
            .map_err(|_| libc::ENOENT)?;

        // Create node for this entry
        let node = {
            let node_cache = self.node_cache.write().await;
            node_cache.insert(path.clone(), info.clone(), parent).await
        };

        let attr = self.file_attr_from_info(&info, node.inode);

        Ok(ReplyEntry {
            ttl: self.cache_ttl,
            attr,
            generation: node.generation,
        })
    }

    /// Get file attributes
    async fn getattr(&self, _req: Request, ino: u64, _fh: Option<u64>, _flags: u32) -> Result<ReplyAttr> {
        if ino == ROOT_INODE {
            // Root directory
            let ts = Timestamp {
                sec: Utc::now().timestamp(),
                nsec: Utc::now().timestamp_subsec_nanos(),
            };

            #[cfg(target_os = "macos")]
            let attr = FileAttr {
                ino: ROOT_INODE,
                size: 4096,
                blocks: 1,
                atime: ts,
                mtime: ts,
                ctime: ts,
                crtime: ts,
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: unsafe { libc::getuid() } as u32,
                gid: unsafe { libc::getgid() } as u32,
                rdev: 0,
                blksize: 4096,
                flags: 0,
            };

            #[cfg(not(target_os = "macos"))]
            let attr = FileAttr {
                ino: ROOT_INODE,
                size: 4096,
                blocks: 1,
                atime: ts,
                mtime: ts,
                ctime: ts,
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: unsafe { libc::getuid() } as u32,
                gid: unsafe { libc::getgid() } as u32,
                rdev: 0,
                blksize: 4096,
            };

            return Ok(ReplyAttr {
                ttl: self.cache_ttl,
                attr,
            });
        }

        // Get node from cache
        let node_cache = self.node_cache.read().await;
        let node = node_cache.get(ino).await.ok_or(libc::ENOENT)?;
        drop(node_cache);

        // Get file info
        let info = self.get_file_info(&node.path).await
            .map_err(|_| libc::ENOENT)?;

        let attr = self.file_attr_from_info(&info, ino);

        Ok(ReplyAttr {
            ttl: self.cache_ttl,
            attr,
        })
    }

    /// Read directory
    async fn readdir<'a>(
        &'a self,
        _req: Request,
        ino: u64,
        _fh: u64,
        offset: i64,
    ) -> Result<ReplyDirectory<Self::DirEntryStream<'a>>> {
        let path = if ino == ROOT_INODE {
            "/".to_string()
        } else {
            let node_cache = self.node_cache.read().await;
            let node = node_cache.get(ino).await.ok_or(libc::ENOENT)?;
            drop(node_cache);
            node.path.clone()
        };

        let files = self.read_dir_cached(&path).await
            .map_err(|_| libc::EIO)?;

        // Create a stream of directory entries
        use futures_util::stream;
        let entries = stream::iter(
            files
                .into_iter()
                .enumerate()
                .skip_while(move |(i, _)| (*i as i64) < offset)
                .map(move |(i, file)| {
                    let kind = if file.is_symlink {
                        FileType::Symlink
                    } else if file.is_dir {
                        FileType::Directory
                    } else {
                        FileType::RegularFile
                    };

                    Ok(DirectoryEntry {
                        inode: (ino + 1 + i as u64) % (1u64 << 48),
                        kind,
                        name: std::ffi::OsString::from(file.name),
                        offset: (i + 2) as i64,
                    })
                }),
        );

        Ok(ReplyDirectory {
            entries: Box::pin(entries),
        })
    }

    /// Open a file
    async fn open(&self, _req: Request, ino: u64, flags: u32) -> Result<ReplyOpen> {
        let node_cache = self.node_cache.read().await;
        let node = node_cache.get(ino).await.ok_or(libc::ENOENT)?;
        drop(node_cache);

        // Open remote handle
        let handle_id = self
            .handles
            .write()
            .await
            .open_remote(&node.path, flags)
            .await
            .map_err(|_| libc::EIO)?;

        Ok(ReplyOpen {
            fh: handle_id,
            flags: libc::O_DIRECT as u32,
        })
    }

    /// Read from file
    async fn read(&self, _req: Request, _ino: u64, fh: u64, offset: u64, size: u32) -> Result<ReplyData> {
        let data = self
            .handles
            .read()
            .await
            .read_remote(fh, offset as i64, size as i64)
            .await
            .map_err(|_| libc::EIO)?;

        Ok(bytes::Bytes::from(data).into())
    }

    /// Write to file
    async fn write(&self, _req: Request, _ino: u64, fh: u64, offset: u64, data: &[u8], _write_flags: u32, _flags: u32) -> Result<ReplyWrite> {
        let written = self
            .handles
            .read()
            .await
            .write_remote(fh, data, offset as i64)
            .await
            .map_err(|_| libc::EIO)?;

        Ok(ReplyWrite {
            written: written as u32,
        })
    }

    /// Release file handle
    async fn release(&self, _req: Request, _ino: u64, fh: u64, _flags: u32, _lock_owner: u64, _flush: bool) -> Result<()> {
        let _ = self.handles.write().await.close(fh).await;
        Ok(())
    }

    /// Create file
    async fn create(&self, _req: Request, parent: u64, name: &std::ffi::OsStr, _mode: u32, flags: u32) -> Result<ReplyCreated> {
        let name_str = name.to_string_lossy();
        let path = if parent == ROOT_INODE {
            format!("/{}", name_str)
        } else {
            let node_cache = self.node_cache.read().await;
            let parent_node = node_cache.get(parent).await.ok_or(libc::ENOENT)?;
            drop(node_cache);
            format!("{}{}", parent_node.path, name_str)
        };

        // Create file on server
        self.client.create(&path).await.map_err(|_| libc::EIO)?;

        // Invalidate parent cache
        self.invalidate_parent_cache(&path);

        // Open handle
        let handle_id = self
            .handles
            .write()
            .await
            .open_remote(&path, flags)
            .await
            .map_err(|_| libc::EIO)?;

        // Get file info
        let info = self.get_file_info(&path).await.map_err(|_| libc::EIO)?;

        // Create node
        let node = {
            let node_cache = self.node_cache.write().await;
            node_cache.insert(path, info.clone(), parent).await
        };

        let attr = self.file_attr_from_info(&info, node.inode);

        Ok(ReplyCreated {
            ttl: self.cache_ttl,
            attr,
            generation: node.generation,
            fh: handle_id,
            flags: libc::O_DIRECT as u32,
        })
    }

    /// Make directory
    async fn mkdir(&self, _req: Request, parent: u64, name: &std::ffi::OsStr, mode: u32, _umask: u32) -> Result<ReplyEntry> {
        let name_str = name.to_string_lossy();
        let path = if parent == ROOT_INODE {
            format!("/{}", name_str)
        } else {
            let node_cache = self.node_cache.read().await;
            let parent_node = node_cache.get(parent).await.ok_or(libc::ENOENT)?;
            drop(node_cache);
            format!("{}{}", parent_node.path, name_str)
        };

        // Create directory on server
        self.client.mkdir(&path, mode).await.map_err(|_| libc::EIO)?;

        // Invalidate parent cache
        self.invalidate_parent_cache(&path);

        // Get file info
        let info = self.get_file_info(&path).await.map_err(|_| libc::EIO)?;

        // Create node
        let node = {
            let node_cache = self.node_cache.write().await;
            node_cache.insert(path, info.clone(), parent).await
        };

        let attr = self.file_attr_from_info(&info, node.inode);

        Ok(ReplyEntry {
            ttl: self.cache_ttl,
            attr,
            generation: node.generation,
        })
    }

    /// Remove file
    async fn unlink(&self, _req: Request, parent: u64, name: &std::ffi::OsStr) -> Result<()> {
        let name_str = name.to_string_lossy();
        let path = if parent == ROOT_INODE {
            format!("/{}", name_str)
        } else {
            let node_cache = self.node_cache.read().await;
            let parent_node = node_cache.get(parent).await.ok_or(libc::ENOENT)?.path.clone();
            drop(node_cache);
            format!("{}{}", parent_node, name_str)
        };

        self.client.remove(&path, false).await.map_err(|_| libc::EIO)?;

        // Invalidate caches
        self.invalidate_cache(&path);
        self.invalidate_parent_cache(&path);

        Ok(())
    }

    /// Remove directory
    async fn rmdir(&self, _req: Request, parent: u64, name: &std::ffi::OsStr) -> Result<()> {
        let name_str = name.to_string_lossy();
        let path = if parent == ROOT_INODE {
            format!("/{}", name_str)
        } else {
            let node_cache = self.node_cache.read().await;
            let parent_node = node_cache.get(parent).await.ok_or(libc::ENOENT)?.path.clone();
            drop(node_cache);
            format!("{}{}", parent_node, name_str)
        };

        self.client.remove(&path, false).await.map_err(|_| libc::EIO)?;

        // Invalidate caches
        self.invalidate_cache(&path);
        self.invalidate_parent_cache(&path);

        Ok(())
    }

    /// Rename file/directory
    async fn rename(&self, _req: Request, parent: u64, name: &std::ffi::OsStr, new_parent: u64, new_name: &std::ffi::OsStr) -> Result<()> {
        let name_str = name.to_string_lossy();
        let newname_str = new_name.to_string_lossy();

        let old_path = if parent == ROOT_INODE {
            format!("/{}", name_str)
        } else {
            let node_cache = self.node_cache.read().await;
            let parent_node = node_cache.get(parent).await.ok_or(libc::ENOENT)?.path.clone();
            drop(node_cache);
            format!("{}{}", parent_node, name_str)
        };

        let new_path = if new_parent == ROOT_INODE {
            format!("/{}", newname_str)
        } else {
            let node_cache = self.node_cache.read().await;
            let new_parent_node = node_cache.get(new_parent).await.ok_or(libc::ENOENT)?.path.clone();
            drop(node_cache);
            format!("{}{}", new_parent_node, newname_str)
        };

        self.client.rename(&old_path, &new_path).await.map_err(|_| libc::EIO)?;

        // Invalidate caches
        self.invalidate_cache(&old_path);
        self.invalidate_cache(&new_path);
        self.invalidate_parent_cache(&old_path);
        self.invalidate_parent_cache(&new_path);

        Ok(())
    }

    /// Set file attributes
    async fn setattr(&self, _req: Request, ino: u64, _fh: Option<u64>, set_attr: SetAttr) -> Result<ReplyAttr> {
        let node_cache = self.node_cache.read().await;
        let node = node_cache.get(ino).await.ok_or(libc::ENOENT)?;
        let path = node.path.clone();
        drop(node_cache);

        // Handle chmod
        if let Some(mode) = set_attr.mode {
            self.client.chmod(&path, mode).await.map_err(|_| libc::EIO)?;
            self.invalidate_cache(&path);
        }

        // Handle truncate
        if let Some(size) = set_attr.size {
            self.client.truncate(&path, size as i64).await.map_err(|_| libc::EIO)?;
            self.invalidate_cache(&path);
        }

        // Return updated attributes
        let info = self.get_file_info(&path).await.map_err(|_| libc::ENOENT)?;
        let attr = self.file_attr_from_info(&info, ino);

        Ok(ReplyAttr {
            ttl: self.cache_ttl,
            attr,
        })
    }

    /// Read symlink target
    async fn readlink(&self, _req: Request, ino: u64) -> Result<ReplyData> {
        let node_cache = self.node_cache.read().await;
        let node = node_cache.get(ino).await.ok_or(libc::ENOENT)?;
        drop(node_cache);

        let target = self.client.readlink(&node.path).await.map_err(|_| libc::EIO)?;
        Ok(bytes::Bytes::from(target).into())
    }

    /// Create symlink
    async fn symlink(&self, _req: Request, parent: u64, name: &std::ffi::OsStr, link: &std::ffi::OsStr) -> Result<ReplyEntry> {
        let name_str = name.to_string_lossy();
        let link_str = link.to_string_lossy();
        let path = if parent == ROOT_INODE {
            format!("/{}", name_str)
        } else {
            let node_cache = self.node_cache.read().await;
            let parent_node = node_cache.get(parent).await.ok_or(libc::ENOENT)?;
            drop(node_cache);
            format!("{}{}", parent_node.path, name_str)
        };

        self.client.symlink(&link_str, &path).await.map_err(|_| libc::EIO)?;

        // Invalidate parent cache
        self.invalidate_parent_cache(&path);

        // Get file info
        let info = self.get_file_info(&path).await.map_err(|_| libc::EIO)?;

        // Create node
        let node = {
            let node_cache = self.node_cache.write().await;
            node_cache.insert(path, info.clone(), parent).await
        };

        let attr = self.file_attr_from_info(&info, node.inode);

        Ok(ReplyEntry {
            ttl: self.cache_ttl,
            attr,
            generation: node.generation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.server_url, "http://localhost:8080/api/v1");
        assert_eq!(config.cache_ttl, std::time::Duration::from_secs(30));
        assert!(!config.debug);
    }

    #[test]
    fn test_mode_to_file_mode() {
        assert_eq!(mode_to_file_mode(0o644), 0o644);
        assert_eq!(mode_to_file_mode(0o755), 0o755);
    }

    #[test]
    fn test_get_stable_mode() {
        let mut info = FileInfo {
            name: "test.txt".to_string(),
            size: 100,
            mode: 0o644,
            mod_time: Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: agfs_sdk::MetaData::default(),
        };

        let mode = get_stable_mode(&info);
        assert!(mode & libc::S_IFREG != 0);
        assert_eq!(mode & 0o777, 0o644);

        info.is_dir = true;
        let mode = get_stable_mode(&info);
        assert!(mode & libc::S_IFDIR != 0);

        info.is_symlink = true;
        info.is_dir = false;
        let mode = get_stable_mode(&info);
        assert!(mode & libc::S_IFLNK != 0);
    }

    #[test]
    fn test_get_parent_path() {
        assert_eq!(get_parent_path("/"), "");
        assert_eq!(get_parent_path("/file"), "/");
        assert_eq!(get_parent_path("/dir/file"), "/dir");
        assert_eq!(get_parent_path("/a/b/c"), "/a/b");
    }
}
