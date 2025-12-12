package fusefs

import (
	"fmt"
	"strings"
	"sync"
	"sync/atomic"

	agfs "github.com/c4pt0r/agfs/agfs-sdk/go"
)

// handleType indicates whether a handle is remote (server-side) or local (client-side fallback)
type handleType int

const (
	handleTypeRemote handleType = iota // Server supports HandleFS
	handleTypeLocal                    // Server doesn't support HandleFS, use local wrapper
)

// handleInfo stores information about an open handle
type handleInfo struct {
	htype      handleType
	agfsHandle int64 // For remote handles: server-side handle ID
	path       string
	flags      agfs.OpenFlag
	mode       uint32
	// Write buffer for local handles - accumulates writes until Close
	writeBuffer []byte
	dirty       bool // true if writeBuffer has data to be flushed
}

// HandleManager manages the mapping between FUSE handles and AGFS handles
type HandleManager struct {
	client *agfs.Client
	mu     sync.RWMutex
	// Map FUSE handle ID to handle info
	handles map[uint64]*handleInfo
	// Counter for generating unique FUSE handle IDs
	nextHandle uint64
}

// NewHandleManager creates a new handle manager
func NewHandleManager(client *agfs.Client) *HandleManager {
	return &HandleManager{
		client:     client,
		handles:    make(map[uint64]*handleInfo),
		nextHandle: 1,
	}
}

// Open opens a file and returns a FUSE handle ID
// If the server supports HandleFS, it uses server-side handles
// Otherwise, it falls back to local handle management
func (hm *HandleManager) Open(path string, flags agfs.OpenFlag, mode uint32) (uint64, error) {
	// Try to open handle on server first
	agfsHandle, err := hm.client.OpenHandle(path, flags, mode)

	// Generate FUSE handle ID
	fuseHandle := atomic.AddUint64(&hm.nextHandle, 1)

	hm.mu.Lock()
	defer hm.mu.Unlock()

	if err != nil {
		// Check if error is because HandleFS is not supported
		if isHandleNotSupportedError(err) {
			// Fall back to local handle management
			hm.handles[fuseHandle] = &handleInfo{
				htype: handleTypeLocal,
				path:  path,
				flags: flags,
				mode:  mode,
			}
			return fuseHandle, nil
		}
		return 0, fmt.Errorf("failed to open handle: %w", err)
	}

	// Server supports HandleFS
	hm.handles[fuseHandle] = &handleInfo{
		htype:      handleTypeRemote,
		agfsHandle: agfsHandle,
		path:       path,
		flags:      flags,
		mode:       mode,
	}

	return fuseHandle, nil
}

// Close closes a handle
func (hm *HandleManager) Close(fuseHandle uint64) error {
	hm.mu.Lock()
	info, ok := hm.handles[fuseHandle]
	if !ok {
		hm.mu.Unlock()
		return fmt.Errorf("handle %d not found", fuseHandle)
	}
	delete(hm.handles, fuseHandle)
	hm.mu.Unlock()

	// Remote handles: close on server
	if info.htype == handleTypeRemote {
		if err := hm.client.CloseHandle(info.agfsHandle); err != nil {
			return fmt.Errorf("failed to close handle: %w", err)
		}
		return nil
	}

	// Local handles: flush write buffer if dirty
	if info.dirty && len(info.writeBuffer) > 0 {
		_, err := hm.client.Write(info.path, info.writeBuffer)
		if err != nil {
			return fmt.Errorf("failed to flush write buffer: %w", err)
		}
	}

	return nil
}

// Read reads data from a handle
func (hm *HandleManager) Read(fuseHandle uint64, offset int64, size int) ([]byte, error) {
	hm.mu.RLock()
	info, ok := hm.handles[fuseHandle]
	hm.mu.RUnlock()

	if !ok {
		return nil, fmt.Errorf("handle %d not found", fuseHandle)
	}

	if info.htype == handleTypeRemote {
		// Use server-side handle
		data, err := hm.client.ReadHandle(info.agfsHandle, offset, size)
		if err != nil {
			return nil, fmt.Errorf("failed to read handle: %w", err)
		}
		return data, nil
	}

	// Local handle: use direct Read API from SDK
	data, err := hm.client.Read(info.path, offset, int64(size))
	if err != nil {
		return nil, fmt.Errorf("failed to read file: %w", err)
	}
	return data, nil
}

// Write writes data to a handle
func (hm *HandleManager) Write(fuseHandle uint64, data []byte, offset int64) (int, error) {
	hm.mu.Lock()
	info, ok := hm.handles[fuseHandle]
	if !ok {
		hm.mu.Unlock()
		return 0, fmt.Errorf("handle %d not found", fuseHandle)
	}

	if info.htype == handleTypeRemote {
		hm.mu.Unlock()
		// Use server-side handle (write directly)
		written, err := hm.client.WriteHandle(info.agfsHandle, data, offset)
		if err != nil {
			return 0, fmt.Errorf("failed to write handle: %w", err)
		}
		return written, nil
	}

	// Local handle: buffer the write data
	// This ensures all writes are accumulated and sent as one request on Close
	// which is critical for queuefs enqueue to work correctly with large messages
	if offset == 0 && len(info.writeBuffer) == 0 {
		// First write at offset 0: initialize buffer
		info.writeBuffer = make([]byte, len(data))
		copy(info.writeBuffer, data)
	} else if offset == int64(len(info.writeBuffer)) {
		// Sequential append
		info.writeBuffer = append(info.writeBuffer, data...)
	} else if offset < int64(len(info.writeBuffer)) {
		// Overwrite within existing buffer
		endOffset := offset + int64(len(data))
		if endOffset > int64(len(info.writeBuffer)) {
			// Extend buffer if needed
			newBuf := make([]byte, endOffset)
			copy(newBuf, info.writeBuffer)
			info.writeBuffer = newBuf
		}
		copy(info.writeBuffer[offset:], data)
	} else {
		// Gap in write - extend with zeros
		newBuf := make([]byte, offset+int64(len(data)))
		copy(newBuf, info.writeBuffer)
		copy(newBuf[offset:], data)
		info.writeBuffer = newBuf
	}
	info.dirty = true
	hm.mu.Unlock()

	return len(data), nil
}

// Sync syncs a handle
func (hm *HandleManager) Sync(fuseHandle uint64) error {
	hm.mu.Lock()
	info, ok := hm.handles[fuseHandle]
	if !ok {
		hm.mu.Unlock()
		return fmt.Errorf("handle %d not found", fuseHandle)
	}

	// Remote handles: sync on server
	if info.htype == handleTypeRemote {
		hm.mu.Unlock()
		if err := hm.client.SyncHandle(info.agfsHandle); err != nil {
			return fmt.Errorf("failed to sync handle: %w", err)
		}
		return nil
	}

	// Local handles: flush write buffer if dirty
	if info.dirty && len(info.writeBuffer) > 0 {
		data := info.writeBuffer
		info.writeBuffer = nil
		info.dirty = false
		hm.mu.Unlock()

		_, err := hm.client.Write(info.path, data)
		if err != nil {
			return fmt.Errorf("failed to sync write buffer: %w", err)
		}
		return nil
	}

	hm.mu.Unlock()
	return nil
}

// CloseAll closes all open handles
func (hm *HandleManager) CloseAll() error {
	hm.mu.Lock()
	handles := make(map[uint64]*handleInfo)
	for k, v := range hm.handles {
		handles[k] = v
	}
	hm.handles = make(map[uint64]*handleInfo)
	hm.mu.Unlock()

	var lastErr error
	for _, info := range handles {
		if info.htype == handleTypeRemote {
			if err := hm.client.CloseHandle(info.agfsHandle); err != nil {
				lastErr = err
			}
		}
	}

	return lastErr
}

// Count returns the number of open handles
func (hm *HandleManager) Count() int {
	hm.mu.RLock()
	defer hm.mu.RUnlock()
	return len(hm.handles)
}

// isHandleNotSupportedError checks if the error indicates HandleFS is not supported
func isHandleNotSupportedError(err error) bool {
	if err == nil {
		return false
	}
	errStr := err.Error()
	return strings.Contains(errStr, "does not support file handles") ||
		strings.Contains(errStr, "not support") ||
		strings.Contains(errStr, "HandleFS")
}
