# AGFS Server API Reference

This document provides a comprehensive reference for the AGFS Server RESTful API. All endpoints are prefixed with `/api/v1`.

## Response Formats

### Success Response
Most successful write/modification operations return a JSON object with a message:
```json
{
  "message": "operation successful"
}
```

### Error Response
Errors are returned with an appropriate HTTP status code and a JSON object:
```json
{
  "error": "error message description"
}
```

### File Info Object
Used in `stat` and directory listing responses:
```json
{
  "name": "filename",
  "size": 1024,
  "mode": 420,             // File mode (decimal)
  "modTime": "2023-10-27T10:00:00Z",
  "isDir": false,
  "meta": {                // Optional metadata
    "name": "plugin_name",
    "type": "file_type"
  }
}
```

---

## File Operations

### Read File
Read content from a file.

**Endpoint:** `GET /api/v1/files`

**Query Parameters:**
- `path` (required): Absolute path to the file.
- `offset` (optional): Byte offset to start reading from.
- `size` (optional): Number of bytes to read. Defaults to reading until EOF.
- `stream` (optional): Set to `true` for streaming response (Chunked Transfer Encoding).

**Response:**
- Binary file content (`application/octet-stream`).

**Example:**
```bash
curl "http://localhost:8080/api/v1/files?path=/memfs/data.txt"
```

### Write File
Write content to a file. Overwrites existing content.

**Endpoint:** `PUT /api/v1/files`

**Query Parameters:**
- `path` (required): Absolute path to the file.

**Body:** Raw file content.

**Example:**
```bash
curl -X PUT "http://localhost:8080/api/v1/files?path=/memfs/data.txt" -d "Hello World"
```

### Create Empty File
Create a new empty file.

**Endpoint:** `POST /api/v1/files`

**Query Parameters:**
- `path` (required): Absolute path to the file.

**Example:**
```bash
curl -X POST "http://localhost:8080/api/v1/files?path=/memfs/empty.txt"
```

### Delete File
Delete a file or directory.

**Endpoint:** `DELETE /api/v1/files`

**Query Parameters:**
- `path` (required): Absolute path.
- `recursive` (optional): Set to `true` to delete directories recursively.

**Example:**
```bash
curl -X DELETE "http://localhost:8080/api/v1/files?path=/memfs/data.txt"
```

### Touch File
Update a file's timestamp or create it if it doesn't exist.

**Endpoint:** `POST /api/v1/touch`

**Query Parameters:**
- `path` (required): Absolute path.

**Example:**
```bash
curl -X POST "http://localhost:8080/api/v1/touch?path=/memfs/data.txt"
```

### Calculate Digest
Calculate the hash digest of a file.

**Endpoint:** `POST /api/v1/digest`

**Body:**
```json
{
  "algorithm": "xxh3",  // or "md5"
  "path": "/memfs/large_file.iso"
}
```

**Response:**
```json
{
  "algorithm": "xxh3",
  "path": "/memfs/large_file.iso",
  "digest": "a1b2c3d4e5f6..."
}
```

**Example:**
```bash
curl -X POST "http://localhost:8080/api/v1/digest" \
  -H "Content-Type: application/json" \
  -d '{"algorithm": "xxh3", "path": "/memfs/large_file.iso"}'
```

### Grep / Search
Search for a regex pattern within files.

**Endpoint:** `POST /api/v1/grep`

**Body:**
```json
{
  "path": "/memfs/logs",
  "pattern": "error|warning",
  "recursive": true,
  "case_insensitive": true,
  "stream": false
}
```

**Response (Normal):**
```json
{
  "matches": [
    {
      "file": "/memfs/logs/app.log",
      "line": 42,
      "content": "ERROR: Connection failed"
    }
  ],
  "count": 1
}
```

**Response (Stream):**
Returns NDJSON (Newline Delimited JSON) stream of matches.

**Example:**
```bash
curl -X POST "http://localhost:8080/api/v1/grep" \
  -H "Content-Type: application/json" \
  -d '{"path": "/memfs/logs", "pattern": "error|warning", "recursive": true, "case_insensitive": true}'
```

---

## Directory Operations

### List Directory
Get a list of files in a directory.

**Endpoint:** `GET /api/v1/directories`

**Query Parameters:**
- `path` (optional): Absolute path. Defaults to `/`.

**Response:**
```json
{
  "files": [
    { "name": "file1.txt", "size": 100, "isDir": false, ... },
    { "name": "dir1", "size": 0, "isDir": true, ... }
  ]
}
```

**Example:**
```bash
curl "http://localhost:8080/api/v1/directories?path=/memfs"
```

### Create Directory
Create a new directory.

**Endpoint:** `POST /api/v1/directories`

**Query Parameters:**
- `path` (required): Absolute path.
- `mode` (optional): Octal mode (e.g., `0755`).

**Example:**
```bash
curl -X POST "http://localhost:8080/api/v1/directories?path=/memfs/newdir"
```

---

## Metadata & Attributes

### Get File Statistics
Get metadata for a file or directory.

**Endpoint:** `GET /api/v1/stat`

**Query Parameters:**
- `path` (required): Absolute path.

**Response:** Returns a [File Info Object](#file-info-object).

**Example:**
```bash
curl "http://localhost:8080/api/v1/stat?path=/memfs/data.txt"
```

### Rename
Rename or move a file/directory.

**Endpoint:** `POST /api/v1/rename`

**Query Parameters:**
- `path` (required): Current absolute path.

**Body:**
```json
{
  "newPath": "/memfs/new_name.txt"
}
```

**Example:**
```bash
curl -X POST "http://localhost:8080/api/v1/rename?path=/memfs/old_name.txt" \
  -H "Content-Type: application/json" \
  -d '{"newPath": "/memfs/new_name.txt"}'
```

### Change Permissions (Chmod)
Change file mode bits.

**Endpoint:** `POST /api/v1/chmod`

**Query Parameters:**
- `path` (required): Absolute path.

**Body:**
```json
{
  "mode": 420  // 0644 in decimal
}
```

**Example:**
```bash
curl -X POST "http://localhost:8080/api/v1/chmod?path=/memfs/data.txt" \
  -H "Content-Type: application/json" \
  -d '{"mode": 420}'
```

---

## Plugin Management

### List Mounts
List all currently mounted plugins.

**Endpoint:** `GET /api/v1/mounts`

**Response:**
```json
{
  "mounts": [
    {
      "path": "/memfs",
      "pluginName": "memfs",
      "config": {}
    }
  ]
}
```

**Example:**
```bash
curl "http://localhost:8080/api/v1/mounts"
```

### Mount Plugin
Mount a new plugin instance.

**Endpoint:** `POST /api/v1/mount`

**Body:**
```json
{
  "fstype": "memfs",      // Plugin type name
  "path": "/my_memfs",    // Mount path
  "config": {             // Plugin-specific configuration
    "init_dirs": ["/tmp"]
  }
}
```

**Example:**
```bash
curl -X POST "http://localhost:8080/api/v1/mount" \
  -H "Content-Type: application/json" \
  -d '{"fstype": "memfs", "path": "/my_memfs", "config": {"init_dirs": ["/tmp"]}}'
```

### Unmount Plugin
Unmount a plugin.

**Endpoint:** `POST /api/v1/unmount`

**Body:**
```json
{
  "path": "/my_memfs"
}
```

**Example:**
```bash
curl -X POST "http://localhost:8080/api/v1/unmount" \
  -H "Content-Type: application/json" \
  -d '{"path": "/my_memfs"}'
```

### List Plugins
List all available (loaded) plugins, including external ones.

**Endpoint:** `GET /api/v1/plugins`

**Response:**
```json
{
  "plugins": [
    {
      "name": "memfs",
      "is_external": false,
      "mounted_paths": [...]
    },
    {
      "name": "hellofs-c",
      "is_external": true,
      "library_path": "./plugins/hellofs.so"
    }
  ]
}
```

**Example:**
```bash
curl "http://localhost:8080/api/v1/plugins"
```

### Load External Plugin
Load a dynamic library plugin (.so/.dylib/.dll) or WASM plugin.

**Endpoint:** `POST /api/v1/plugins/load`

**Body:**
```json
{
  "library_path": "./plugins/myplugin.so"
}
```
*Note: `library_path` can also be a URL (`http://...`) or an AGFS path (`agfs://...`) to load remote plugins.*

**Example:**
```bash
curl -X POST "http://localhost:8080/api/v1/plugins/load" \
  -H "Content-Type: application/json" \
  -d '{"library_path": "./plugins/myplugin.so"}'
```

### Unload External Plugin
Unload a previously loaded external plugin.

**Endpoint:** `POST /api/v1/plugins/unload`

**Body:**
```json
{
  "library_path": "./plugins/myplugin.so"
}
```

**Example:**
```bash
curl -X POST "http://localhost:8080/api/v1/plugins/unload" \
  -H "Content-Type: application/json" \
  -d '{"library_path": "./plugins/myplugin.so"}'
```

---

## System

### Health Check
Check server status and version.

**Endpoint:** `GET /api/v1/health`

**Response:**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "gitCommit": "abcdef",
  "buildTime": "2023-..."
}
```

**Example:**
```bash
curl "http://localhost:8080/api/v1/health"
```
