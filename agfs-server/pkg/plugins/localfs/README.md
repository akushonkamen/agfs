# LocalFS Plugin - Local File System Mount

This plugin mounts a local directory into the AGFS virtual file system.

## Features
- Mount any local directory into AGFS
- Full POSIX file system operations
- Direct access to local files and directories
- Preserves file permissions and timestamps
- Efficient file operations (no copying)

## Configuration

Basic configuration:
```toml  
[plugins.localfs]
enabled = true
path = "/local"

  [plugins.localfs.config]
  local_dir = "/path/to/local/directory"
```

Multiple local mounts:
```toml
[plugins.localfs_home]
enabled = true
path = "/home"

  [plugins.localfs_home.config]
  local_dir = "/Users/username"

[plugins.localfs_data]
enabled = true
path = "/data"

  [plugins.localfs_data.config]
  local_dir = "/var/data"
```
## Current Mount
Base Path: %s

## Usage

List directory:
```bash  
agfs ls /local
```

Read a file:
```bash
agfs cat /local/file.txt
```

Write to a file:
```bash
agfs write /local/file.txt "Hello, World!"
```

Create a directory:
```bash
agfs mkdir /local/newdir
```

Remove a file:
```bash
agfs rm /local/file.txt
```

Remove directory recursively:
```bash
agfs rm -r /local/olddir
```

Move/rename:
```bash
agfs mv /local/old.txt /local/new.txt
```

Change permissions:
```bash
agfs chmod 755 /local/script.sh
```

## Examples

```bash
# Basic file operations
agfs:/> ls /local
file1.txt  dir1/  dir2/

agfs:/> cat /local/file1.txt
Hello from local filesystem!

agfs:/> echo "new content" > /local/file2.txt
Written 12 bytes to /local/file2.txt

# Directory operations
agfs:/> mkdir /local/newdir
agfs:/> ls /local
file1.txt  file2.txt  dir1/  dir2/  newdir/
```

## Notes
- Changes are directly applied to the local file system
- File permissions are preserved and can be modified
- Symlinks are followed by default
- Be careful with rm -r as it permanently deletes files

## Use Case
- Access local configuration files
- Process local data files
- Integrate with existing file-based workflows
- Development and testing with local data
- Backup and sync operations

## Advantages
- No data copying overhead
- Direct access to local files
- Preserves all file system metadata
- Supports all standard file operations
- Efficient for large files

## License

Apache License 2.0
