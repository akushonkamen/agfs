# SQLFS Plugin - Database-backed File System

This plugin provides a persistent file system backed by database storage.

## Features
- Persistent storage (survives server restarts)
- Full POSIX-like file system operations
- Multiple database backends (SQLite, TiDB)
- Efficient database-backed storage
- ACID transactions
- Supports files and directories
- Maximum file size: 5MB per file

## Dynamic Mounting With AGFS Shell

Interactive shell - SQLite:
```bash  
agfs:/> mount sqlfs /db backend=sqlite db_path=/tmp/mydata.db
agfs:/> mount sqlfs /persistent backend=sqlite db_path=./storage.db
agfs:/> mount sqlfs /cache backend=sqlite db_path=/tmp/cache.db cache_enabled=true cache_max_size=2000
```

Interactive shell - TiDB:
```bash  
agfs:/> mount sqlfs /tidb backend=tidb dsn="user:pass@tcp(localhost:4000)/database"
agfs:/> mount sqlfs /cloud backend=tidb user=root password=mypass host=tidb-server.com port=4000 database=agfs_data enable_tls=true
```

Direct command - SQLite:
```bash  
uv run agfs mount sqlfs /db backend=sqlite db_path=/tmp/test.db
```

Direct command - TiDB:
```bash  
uv run agfs mount sqlfs /tidb backend=tidb dsn="user:pass@tcp(host:4000)/db"
```

## CONFIGURATION PARAMETERS:

### Required (SQLite):
- `backend`: "sqlite" or "sqlite3"
- `db_path`: Path to SQLite database file (created if doesn't exist)

### Required (TiDB) - Option 1 (DSN):
- `backend`: "tidb"
- `dsn`: Full database connection string (e.g., `user:pass@tcp(host:4000)/db`)

### Required (TiDB) - Option 2 (Individual parameters):
- `backend`: "tidb"
- `user`: Database username
- `password`: Database password
- `host`: Database host
- `port`: Database port (typically `4000`)
- `database`: Database name

### Optional (All backends):
- `cache_enabled`: Enable directory listing cache (default: true)
- `cache_max_size`: Maximum cached entries (default: 1000)
- `cache_ttl_seconds`: Cache TTL in seconds (default: 5)
- `enable_tls`: Enable TLS for TiDB (default: false)
- `tls_server_name`: TLS server name for TiDB

## Examples
```bash
# Multiple databases
agfs:/> mount sqlfs /local backend=sqlite db_path=local.db
agfs:/> mount sqlfs /shared backend=tidb dsn="user:pass@tcp(shared-db:4000)/agfs"

# With custom cache settings
agfs:/> mount sqlfs /fast backend=sqlite db_path=fast.db cache_enabled=true cache_max_size=5000 cache_ttl_seconds=10
```

## Static Configuration (config.yaml):

Alternative to dynamic mounting - configure in server config file:

SQLite Backend (Local Testing):
```yaml
plugins:
  sqlfs:
    enabled: true
    path: /sqlfs
    config:
      backend: sqlite # or sqlite3
      db_path: sqlfs.db

      # Optional cache settings (enabled by default)
      cache_enabled: true # Enable/disable directory listing cache
      cache_max_size: 1000 # Maximum number of cached entries (default: 1000)
      cache_ttl_seconds: 5 # Cache entry TTL in seconds (default: 5)
```

TiDB Backend (Production):
```yaml
plugins:
  sqlfs:
    enabled: true
    path: /sqlfs
    config:
      backend: tidb

      # For TiDB Cloud (TLS required):
      user: 3YdGXuXNdAEmP1f.root
      password: your_password
      host: gateway01.us-west-2.prod.aws.tidbcloud.com
      port: "4000"
      database: baas
      enable_tls: true
      tls_server_name: gateway01.us-west-2.prod.aws.tidbcloud.com

      # Or use DSN with TLS:
      # dsn: "user:password@tcp(host:4000)/database?charset=utf8mb4&parseTime=True&tls=tidb"
```

## Usage

Create a directory:
```bash    
agfs mkdir /sqlfs/mydir
```

Create a file:
```bash
agfs write /sqlfs/mydir/file.txt "Hello, World!"
```

Read a file:
```bash
agfs cat /sqlfs/mydir/file.txt
```

List directory:
```bash
agfs ls /sqlfs/mydir
```

Get file info:
```bash
agfs stat /sqlfs/mydir/file.txt
```
Rename file:
```bash
agfs mv /sqlfs/mydir/file.txt /sqlfs/mydir/newfile.txt
```

Change permissions:
```bash
agfs chmod 755 /sqlfs/mydir/file.txt
```

Remove file:
```bash
agfs rm /sqlfs/mydir/file.txt
```

Remove directory (must be empty):
```bash
agfs rm /sqlfs/mydir
```

Remove directory recursively:
```bash
agfs rm -r /sqlfs/mydir
```

## Examples

```bash  
# Create directory structure
agfs:/> mkdir /sqlfs/data
agfs:/> mkdir /sqlfs/data/logs

# Write files
agfs:/> echo "Configuration data" > /sqlfs/data/config.txt
agfs:/> echo "Log entry" > /sqlfs/data/logs/app.log

# Read files
agfs:/> cat /sqlfs/data/config.txt
```
  
Configuration data
```bash
# List directory
agfs:/> ls /sqlfs/data
config.txt
logs/
```

## Advantages
- Data persists across server restarts
- Efficient storage with database compression
- Transaction safety (ACID properties)
- Query capabilities (can be extended)
- Backup friendly (single database file)
- Fast directory listing with LRU cache (improves shell completion)

## Use Cases
- Persistent configuration storage
- Log file storage
- Document management
- Application data storage
- Backup and archival
- Development and testing with persistent data

## Technical Details
- Database: SQLite3/TiDB (MySQL-compatible)
- Journal mode: WAL (Write-Ahead Logging) for SQLite
- Schema: Single table with path, metadata, and blob data
- Concurrent reads supported
- Write serialization via mutex
- Path normalization and validation
- LRU cache for directory listings (configurable TTL and size)
- Automatic cache invalidation on modifications

## Limitations
- Maximum file size: 5MB per file
- Not suitable for large files (use MemFS or StreamFS for larger data)
- Write operations are serialized
- No file locking mechanism
- No sparse file support
- No streaming support (use StreamFS for real-time streaming)

## License

Apache License 2.0
