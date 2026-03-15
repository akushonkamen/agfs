//! Traffic monitor for AGFS server
//!
//! This module provides atomic counters for tracking server traffic.
//! Based on the Go implementation in `agfs-server/pkg/monitor/monitor.go`.

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

/// Traffic monitor for tracking server statistics
///
/// Tracks various metrics about file system operations and data transfer.
#[derive(Debug)]
pub struct TrafficMonitor {
    /// Total bytes read
    total_bytes_read: AtomicU64,
    /// Total bytes written
    total_bytes_written: AtomicU64,
    /// Total number of read operations
    total_reads: AtomicI64,
    /// Total number of write operations
    total_writes: AtomicI64,
    /// Total number of other operations
    total_other_ops: AtomicI64,
}

impl TrafficMonitor {
    /// Create a new traffic monitor
    pub fn new() -> Self {
        Self {
            total_bytes_read: AtomicU64::new(0),
            total_bytes_written: AtomicU64::new(0),
            total_reads: AtomicI64::new(0),
            total_writes: AtomicI64::new(0),
            total_other_ops: AtomicI64::new(0),
        }
    }

    /// Record a read operation
    pub fn record_read(&self, bytes: u64) {
        self.total_bytes_read.fetch_add(bytes, Ordering::Relaxed);
        self.total_reads.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a write operation
    pub fn record_write(&self, bytes: u64) {
        self.total_bytes_written.fetch_add(bytes, Ordering::Relaxed);
        self.total_writes.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an other operation (non-read/write)
    pub fn record_other_op(&self) {
        self.total_other_ops.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the total bytes read
    pub fn get_bytes_read(&self) -> u64 {
        self.total_bytes_read.load(Ordering::Relaxed)
    }

    /// Get the total bytes written
    pub fn get_bytes_written(&self) -> u64 {
        self.total_bytes_written.load(Ordering::Relaxed)
    }

    /// Get the total number of read operations
    pub fn get_reads(&self) -> i64 {
        self.total_reads.load(Ordering::Relaxed)
    }

    /// Get the total number of write operations
    pub fn get_writes(&self) -> i64 {
        self.total_writes.load(Ordering::Relaxed)
    }

    /// Get the total number of other operations
    pub fn get_other_ops(&self) -> i64 {
        self.total_other_ops.load(Ordering::Relaxed)
    }

    /// Reset all counters to zero
    pub fn reset(&self) {
        self.total_bytes_read.store(0, Ordering::Relaxed);
        self.total_bytes_written.store(0, Ordering::Relaxed);
        self.total_reads.store(0, Ordering::Relaxed);
        self.total_writes.store(0, Ordering::Relaxed);
        self.total_other_ops.store(0, Ordering::Relaxed);
    }

    /// Get all statistics as a struct
    pub fn get_stats(&self) -> TrafficStats {
        TrafficStats {
            bytes_read: self.get_bytes_read(),
            bytes_written: self.get_bytes_written(),
            reads: self.get_reads(),
            writes: self.get_writes(),
            other_ops: self.get_other_ops(),
        }
    }
}

impl Default for TrafficMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Traffic statistics snapshot
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrafficStats {
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Total number of read operations
    pub reads: i64,
    /// Total number of write operations
    pub writes: i64,
    /// Total number of other operations
    pub other_ops: i64,
}

/// Shared traffic monitor handle
pub type SharedTrafficMonitor = Arc<TrafficMonitor>;
