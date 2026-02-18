# HTTPFS Plugin - HTTP File Server for AGFS Paths

This plugin serves a AGFS mount path over HTTP, similar to `python3 -m http.server`.
Unlike serving local files, this exposes any AGFS filesystem (memfs, queuefs, s3fs, etc.) via HTTP.

## Features
- Serve any AGFS path via HTTP (e.g., `/memfs`, `/queuefs`, `/s3fs`)
- Browse files and directories in web browser
- Download files via HTTP
- Pretty HTML directory listings
- Access AGFS virtual filesystems through HTTP
- Read-only HTTP access (modifications should be done through AGFS API)
- Support for dynamic mounting via AGFS Shell mount command

CONFIGURATION:

  Basic configuration:
 ```toml
[plugins.httpfs]
enabled = true
path = "/httpfs"              # This is just a placeholder, not used for serving

  [plugins.httpfs.config]
  agfs_path = "/memfs"         # The AGFS path to serve (e.g., /memfs, /queuefs)
  host = "0.0.0.0"            # Optional, defaults to 0.0.0.0 (all interfaces)
  port = "8000"               # Optional, defaults to 8000
```
  Example - Serve memfs:
```toml  
[plugins.httpfs_mem]
enabled = true
path = "/httpfs_mem"

   [plugins.httpfs_mem.config]
   agfs_path = "/memfs"
   host = "localhost"
   port = "9000"
```

Example - Serve queuefs:
```toml
[plugins.httpfs_queue]
enabled = true
path = "/httpfs_queue"

  [plugins.httpfs_queue.config]
  agfs_path = "/queuefs"
  port = "9001"
```

## Current Configuration
- **AGFS Path**: %s
- **HTTP Server**: http://%s:%s

## Dynamic Mounting

You can dynamically mount httagfs at runtime using AGFS Shell:
```bash
  # In AGFS Shell REPL:
  > mount httagfs /httpfs-demo agfs_path=/memfs port=10000
    plugin mounted

  > mount httagfs /web agfs_path=/local host=localhost port=9000
    plugin mounted

  > mounts
  httagfs on /httpfs-demo (plugin: httpfs, agfs_path=/memfs, port=10000)
  httagfs on /web (plugin: httpfs, agfs_path=/local, host=localhost, port=9000)

  > unmount /httpfs-demo
  Unmounted plugin at /httpfs-demo

  # Via command line:
  agfs mount httagfs /httpfs-demo agfs_path=/memfs port=10000
  agfs mount httagfs /web agfs_path=/local host=localhost port=9000
  agfs unmount /httpfs-demo

  # Via REST API:
  curl -X POST http://localhost:8080/api/v1/mount \
       -H "Content-Type: application/json" \
       -d '{
         "fstype": "httpfs",
         "path": "/httpfs-demo",
         "config": {
           "agfs_path": "/memfs",
           "host": "0.0.0.0",
           "port": "10000"
         }
       }'
```

### Dynamic mounting advantages
- No server restart required
- Mount/unmount on demand
- Multiple instances with different configurations
- Flexible port and path selection

## Usage

Via Web Browser:
  Open: http://localhost:%s
  Browse directories and download files from AGFS

Via curl:
```bash
# List directory
curl http://localhost:%s/

# Download file
curl http://localhost:%s/file.txt

# Access subdirectory
curl http://localhost:%s/subdir/
```
## Examples
```bash
# Serve memfs on port 9000
http://localhost:9000 -> shows contents of /memfs

# Serve queuefs on port 9001
http://localhost:9001 -> shows contents of /queuefs

# Access files in browser
Open http://localhost:%s in your browser
Click on files to download
Click on directories to browse
```

## Notes
- The HTTP server starts automatically when the plugin is initialized
- Files are served with proper MIME types
- Directory listings are formatted as pretty HTML
- httagfs provides HTTP read-only access to AGFS paths
- To modify files, use the AGFS API directly
- Multiple httagfs instances can serve different AGFS paths on different ports

## Use Case
- Expose in-memory files (memfs) via HTTP for easy access
- Browse queue contents (queuefs) in a web browser
- Share S3 files (s3fs) through a simple HTTP interface
- Provide web access to any AGFS filesystem
- Quick file sharing without setting up separate web servers
- Debug and inspect AGFS filesystems visually

## Advantages
- Works with any AGFS filesystem (not just local files)
- Simple HTTP interface for complex backends
- Multiple instances can serve different paths
- No data duplication - serves directly from AGFS
- Lightweight and fast

  
## License

Apache License 2.0
