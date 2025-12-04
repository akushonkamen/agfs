# AGFS WASM FFI

A safe, high-level Rust SDK for building AGFS filesystem plugins in WebAssembly.

## Overview

This library provides a complete abstraction layer for developing AGFS plugins in Rust/WASM. It handles all the low-level details of the WASM interface and provides a clean, type-safe API.

## Features

- **Safe Abstractions**: Minimal unsafe code, all hidden behind safe interfaces
- **Easy to Use**: Simply implement a trait and export with a macro
- **Type-Safe**: Strong typing for all filesystem operations
- **Host FS Access**: Access to the host filesystem from WASM
- **HTTP Client**: Make HTTP requests to external services
- **Flexible**: Support for both read-only and read-write filesystems

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
agfs-wasm-ffi = { path = "../agfs-wasm-ffi" }

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
```

## Example: Read-Only Filesystem

```rust
use agfs_wasm_ffi::prelude::*;

#[derive(Default)]
struct HelloFS;

impl ReadOnlyFileSystem for HelloFS {
    fn name(&self) -> &str {
        "hellofs"
    }

    fn readme(&self) -> &str {
        "A simple Hello World filesystem"
    }

    fn read(&self, path: &str, _offset: i64, _size: i64) -> Result<Vec<u8>> {
        match path {
            "/hello.txt" => Ok(b"Hello, World!\n".to_vec()),
            _ => Err(Error::NotFound),
        }
    }

    fn stat(&self, path: &str) -> Result<FileInfo> {
        match path {
            "/" => Ok(FileInfo::dir("", 0o755)),
            "/hello.txt" => Ok(FileInfo::file("hello.txt", 14, 0o644)),
            _ => Err(Error::NotFound),
        }
    }

    fn readdir(&self, path: &str) -> Result<Vec<FileInfo>> {
        match path {
            "/" => Ok(vec![FileInfo::file("hello.txt", 14, 0o644)]),
            _ => Err(Error::NotFound),
        }
    }
}

export_plugin!(HelloFS);
```

## Example: Read-Write Filesystem

```rust
use agfs_wasm_ffi::prelude::*;
use std::collections::HashMap;
use std::cell::RefCell;

#[derive(Default)]
struct MemFS {
    files: RefCell<HashMap<String, Vec<u8>>>,
}

impl FileSystem for MemFS {
    fn name(&self) -> &str {
        "memfs"
    }

    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>> {
        let files = self.files.borrow();
        let data = files.get(path).ok_or(Error::NotFound)?;
        // ... handle offset and size
        Ok(data.clone())
    }

    fn write(&mut self, path: &str, data: &[u8]) -> Result<Vec<u8>> {
        self.files.borrow_mut().insert(path.to_string(), data.to_vec());
        Ok(vec![])
    }

    // ... implement other methods
}

export_plugin!(MemFS);
```

## Host Filesystem Access

Access the host filesystem from your WASM plugin:

```rust
use agfs_wasm_ffi::prelude::*;

// Read from host filesystem
let data = HostFS::read("/path/on/host/file.txt", 0, -1)?;

// Write to host filesystem
HostFS::write("/path/on/host/output.txt", b"Hello")?;

// Get file info
let info = HostFS::stat("/path/on/host/file.txt")?;

// List directory
let entries = HostFS::readdir("/path/on/host/dir")?;
```

## HTTP Client

Make HTTP requests from your WASM plugin:

```rust
use agfs_wasm_ffi::prelude::*;

// Simple GET request
let response = Http::get("https://api.example.com/data")?;
let text = response.text()?;

// POST with JSON
let data = serde_json::json!({"key": "value"});
let response = Http::post_json("https://api.example.com/submit", &data)?;

// Custom request
let response = Http::request(
    HttpRequest::post("https://api.example.com/upload")
        .header("Authorization", "Bearer token")
        .body_str("data")
        .timeout(60)
)?;

// Parse JSON response
#[derive(Deserialize)]
struct ApiResponse {
    status: String,
}

let api_response: ApiResponse = response.json()?;
```

## API Reference

### Traits

- **`ReadOnlyFileSystem`**: Implement for read-only filesystems
  - Required: `name()`, `read()`, `stat()`, `readdir()`
  - Optional: `readme()`, `initialize()`

- **`FileSystem`**: Implement for read-write filesystems
  - All ReadOnlyFileSystem methods
  - Additional: `write()`, `create()`, `mkdir()`, `remove()`, etc.

### Types

- **`FileInfo`**: File metadata (name, size, mode, timestamps)
- **`Error`**: Filesystem errors (NotFound, PermissionDenied, etc.)
- **`Config`**: Plugin configuration passed during initialization
- **`HttpRequest`**: HTTP request builder
- **`HttpResponse`**: HTTP response with status, headers, body

### Macros

- **`export_plugin!(Type)`**: Export your filesystem as a WASM plugin

## Building

Build your WASM plugin:

```bash
cargo build --release --target wasm32-unknown-unknown
```

Optimize with `wasm-opt` (optional):

```bash
wasm-opt -Oz target/wasm32-unknown-unknown/release/your_plugin.wasm \
  -o optimized.wasm
```

## Examples

See the `examples/` directory for complete examples:

- **hellofs-wasm**: Simple read-only filesystem with host FS access
- **hackernewsfs-wasm**: Fetches Hacker News stories via HTTP

## License

Apache-2.0
