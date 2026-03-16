//! Caching for FUSE operations
//!
//! Implements TTL-based caches for:
//! - Metadata cache (file attributes)
//! - Directory cache (directory listings)

use ctxfs_sdk::FileInfo;
use dashmap::DashMap;
use std::time::Duration;

/// Cache entry with TTL
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    /// Cached value
    value: T,
    /// Expiration timestamp
    expires_at: std::time::Instant,
}

impl<T> CacheEntry<T> {
    /// Create new cache entry
    fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: std::time::Instant::now() + ttl,
        }
    }

    /// Check if entry is expired
    fn is_expired(&self) -> bool {
        std::time::Instant::now() > self.expires_at
    }
}

/// Metadata cache for file attributes
pub struct MetadataCache {
    /// Cache map: path -> (FileInfo, expires_at)
    cache: DashMap<String, CacheEntry<FileInfo>>,
    /// Default TTL for cache entries
    default_ttl: Duration,
}

impl MetadataCache {
    /// Create new metadata cache
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            default_ttl: ttl,
        }
    }

    /// Get cached metadata for a path
    pub fn get(&self, path: &str) -> Option<FileInfo> {
        if let Some(entry) = self.cache.get(path) {
            if !entry.is_expired() {
                return Some(entry.value.clone());
            } else {
                // Remove expired entry
                self.cache.remove(path);
            }
        }
        None
    }

    /// Insert metadata into cache
    pub fn insert(&self, path: String, info: FileInfo) {
        let entry = CacheEntry::new(info, self.default_ttl);
        self.cache.insert(path, entry);
    }

    /// Insert with custom TTL
    pub fn insert_with_ttl(&self, path: String, info: FileInfo, ttl: Duration) {
        let entry = CacheEntry::new(info, ttl);
        self.cache.insert(path, entry);
    }

    /// Invalidate cache entry for a path
    pub fn invalidate(&self, path: &str) {
        self.cache.remove(path);
    }

    /// Invalidate all entries under a prefix
    pub fn invalidate_prefix(&self, prefix: &str) {
        let mut to_remove = Vec::new();
        for entry in self.cache.iter() {
            let path = entry.key();
            if path.starts_with(prefix) {
                to_remove.push(path.clone());
            }
        }
        for path in to_remove {
            self.cache.remove(&path);
        }
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&self) {
        let mut to_remove = Vec::new();
        for entry in self.cache.iter() {
            if entry.is_expired() {
                to_remove.push(entry.key().clone());
            }
        }
        for path in to_remove {
            self.cache.remove(&path);
        }
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

/// Directory cache for directory listings
pub struct DirectoryCache {
    /// Cache map: path -> (Vec<FileInfo>, expires_at)
    cache: DashMap<String, CacheEntry<Vec<FileInfo>>>,
    /// Default TTL for cache entries
    default_ttl: Duration,
}

impl DirectoryCache {
    /// Create new directory cache
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            default_ttl: ttl,
        }
    }

    /// Get cached directory listing
    pub fn get(&self, path: &str) -> Option<Vec<FileInfo>> {
        if let Some(entry) = self.cache.get(path) {
            if !entry.is_expired() {
                return Some(entry.value.clone());
            } else {
                // Remove expired entry
                self.cache.remove(path);
            }
        }
        None
    }

    /// Insert directory listing into cache
    pub fn insert(&self, path: String, files: Vec<FileInfo>) {
        let entry = CacheEntry::new(files, self.default_ttl);
        self.cache.insert(path, entry);
    }

    /// Insert with custom TTL
    pub fn insert_with_ttl(&self, path: String, files: Vec<FileInfo>, ttl: Duration) {
        let entry = CacheEntry::new(files, ttl);
        self.cache.insert(path, entry);
    }

    /// Invalidate cache entry for a path
    pub fn invalidate(&self, path: &str) {
        self.cache.remove(path);
    }

    /// Invalidate all entries under a prefix
    pub fn invalidate_prefix(&self, prefix: &str) {
        let mut to_remove = Vec::new();
        for entry in self.cache.iter() {
            let path = entry.key();
            if path.starts_with(prefix) {
                to_remove.push(path.clone());
            }
        }
        for path in to_remove {
            self.cache.remove(&path);
        }
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&self) {
        let mut to_remove = Vec::new();
        for entry in self.cache.iter() {
            if entry.is_expired() {
                to_remove.push(entry.key().clone());
            }
        }
        for path in to_remove {
            self.cache.remove(&path);
        }
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxfs_sdk::MetaData;
    use chrono::Utc;

    #[test]
    fn test_metadata_cache_basic() {
        let cache = MetadataCache::new(Duration::from_secs(30));

        // Insert and get
        let info = FileInfo {
            name: "test.txt".to_string(),
            size: 100,
            mode: 0o644,
            mod_time: Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: MetaData::default(),
        };
        cache.insert("/test.txt".to_string(), info.clone());

        let retrieved = cache.get("/test.txt");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test.txt");
    }

    #[test]
    fn test_metadata_cache_invalidate() {
        let cache = MetadataCache::new(Duration::from_secs(30));

        let info = FileInfo {
            name: "test.txt".to_string(),
            size: 100,
            mode: 0o644,
            mod_time: Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: MetaData::default(),
        };
        cache.insert("/test.txt".to_string(), info);

        // Invalidate
        cache.invalidate("/test.txt");
        assert!(cache.get("/test.txt").is_none());
    }

    #[test]
    fn test_metadata_cache_prefix_invalidate() {
        let cache = MetadataCache::new(Duration::from_secs(30));

        let info = FileInfo {
            name: "file".to_string(),
            size: 100,
            mode: 0o644,
            mod_time: Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: MetaData::default(),
        };
        cache.insert("/dir/sub/file.txt".to_string(), info.clone());
        cache.insert("/dir/file.txt".to_string(), info.clone());
        cache.insert("/other/file.txt".to_string(), info);

        // Invalidate prefix
        cache.invalidate_prefix("/dir");

        assert!(cache.get("/dir/sub/file.txt").is_none());
        assert!(cache.get("/dir/file.txt").is_none());
        assert!(cache.get("/other/file.txt").is_some()); // Not under /dir
    }

    #[test]
    fn test_directory_cache_basic() {
        let cache = DirectoryCache::new(Duration::from_secs(30));

        let files = vec![FileInfo {
            name: "file.txt".to_string(),
            size: 100,
            mode: 0o644,
            mod_time: Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: MetaData::default(),
        }];

        cache.insert("/dir".to_string(), files.clone());

        let retrieved = cache.get("/dir");
        assert!(retrieved.is_some());
        let retrieved_files = retrieved.as_ref().unwrap();
        assert_eq!(retrieved_files.len(), 1);
        assert_eq!(retrieved_files[0].name, "file.txt");
    }

    #[test]
    fn test_cache_cleanup() {
        let cache = MetadataCache::new(Duration::from_millis(10));

        let info = FileInfo {
            name: "test.txt".to_string(),
            size: 100,
            mode: 0o644,
            mod_time: Utc::now(),
            is_dir: false,
            is_symlink: false,
            meta: MetaData::default(),
        };
        cache.insert("/test.txt".to_string(), info);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));

        // Cleanup should remove expired entries
        cache.cleanup_expired();
        assert!(cache.get("/test.txt").is_none());
    }
}
