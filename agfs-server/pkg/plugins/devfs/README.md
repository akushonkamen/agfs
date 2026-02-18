# DevFS Plugin - Device File System

This plugin provides standard Unix device files.

## Available Devices
- `/dev/null`  - Null device (discards writes, returns EOF on reads)

## Usage Example
Read from /dev/null:
```bash
cat /dev/null
```

Write to /dev/null (data is discarded):
```bash
echo "test" > /dev/null
```

Use as redirect target:
```bash
command > /dev/null 2>&1
```

## Characteristics
- `/dev/null` always exists
- Reads always return EOF immediately
- Writes are accepted and discarded
- Cannot be deleted, renamed, or modified
