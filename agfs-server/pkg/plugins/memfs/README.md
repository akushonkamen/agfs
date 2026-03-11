# MemFS Plugin - In-Memory File System

This plugin provides a full-featured in-memory file system.

## Dynamic Mounting With AGFS Shell

Interactive shell:
```bash  
agfs:/> mount memfs /mem
agfs:/> mount memfs /tmp
agfs:/> mount memfs /scratch init_dirs='["/home","/tmp","/data"]'
```

Direct command:
```bash 
uv run agfs mount memfs /mem
uv run agfs mount memfs /tmp init_dirs='["/work","/cache"]'
```

## Configuration Parameters

Optional:
- `init_dirs`: Array of directories to create automatically on mount

Examples:
```bash  
agfs:/> mount memfs /workspace init_dirs='["/projects","/builds","/logs"]'
```

## Features
- Standard file system operations (create, read, write, delete)
- Directory support with hierarchical structure
- File permissions (chmod)
- File/directory renaming and moving
- Metadata tracking

## USAGE
Create a file:
```bash
touch /path/to/file
```

Write to a file:
```bash
echo "content" > /path/to/file
```

Read a file:
```bash
cat /path/to/file
```

Create a directory:
```bash
mkdir /path/to/dir
```

List directory:
```bash
ls /path/to/dir
```

Remove file/directory:
```bash
rm /path/to/file
rm -r /path/to/dir
```

Move/rename:
```bash
mv /old/path /new/path
```

Change permissions:
```bash
chmod 755 /path/to/file
```

## Examples

```bash
agfs:/> mkdir /memfs/data
agfs:/> echo "hello" > /memfs/data/file.txt
agfs:/> cat /memfs/data/file.txt
hello
agfs:/> ls /memfs/data
agfs:/> mv /memfs/data/file.txt /memfs/data/renamed.txt
```

## License

Apache License 2.0
