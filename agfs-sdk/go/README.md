# AGFS Go SDK

Go client SDK for AGFS (Abstract Global File System) HTTP API.

## Installation

```bash
go get github.com/c4pt0r/agfs/agfs-sdk/go
```

## Usage

```go
import agfs "github.com/c4pt0r/agfs/agfs-sdk/go"

// Create a new client
client := agfs.NewClient("http://localhost:8080")

// Create a file
err := client.Create("/path/to/file")

// Write to a file
_, err = client.Write("/path/to/file", []byte("hello world"))

// Read from a file
data, err := client.Read("/path/to/file", 0, -1)

// List directory
files, err := client.ReadDir("/path/to/dir")

// Get file info
info, err := client.Stat("/path/to/file")

// Rename/move
err = client.Rename("/old/path", "/new/path")

// Remove
err = client.Remove("/path/to/file")

// Remove recursively
err = client.RemoveAll("/path/to/dir")

// Streaming read
stream, err := client.ReadStream("/path/to/file")
defer stream.Close()

// Search with grep
results, err := client.Grep("/path", "pattern", true, false)

// Calculate digest
digest, err := client.Digest("/path/to/file", "xxh3")
```

## Features

- Full AGFS HTTP API support
- Automatic retry with exponential backoff for network errors
- Streaming support for large files
- Grep and digest operations
- Type-safe API with proper error handling

## Testing

```bash
go test -v
```

## Types

The SDK provides the following types:

- `Client` - Main client for interacting with AGFS
- `FileInfo` - File metadata information
- `MetaData` - Structured metadata for files and directories
- `ErrorResponse` - API error response
- `SuccessResponse` - API success response
- `GrepResponse` - Grep search results
- `DigestResponse` - File digest results

## License

Same as AGFS project.
