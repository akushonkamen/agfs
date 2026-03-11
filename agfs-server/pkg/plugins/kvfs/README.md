# KVFS Plugin - Key-Value Store Service

This plugin provides a key-value store service through a file system interface.

## Structure

```bash  
/keys/     # Directory containing all key-value pairs
/README    # This file
```

## Dynamic Mounting With AGFS Shell

Interactive shell:
```bash
agfs:/> mount kvfs /kv
agfs:/> mount kvfs /cache
```

Direct command:
```bash
uv run agfs mount kvfs /kv
uv run agfs mount kvfs /store
```

## Configuration Parameters

Optional:
- `initial_data`: Map of initial key-value pairs to populate on mount

Example with initial data:
```bash
agfs:/> mount kvfs /config initial_data='{"app":"myapp","version":"1.0"}'
```

## Usage

Set a key-value pair:
```bash
echo "value" > /keys/<key>
```

Get a value:
```bash
cat /keys/<key>
```

List all keys:
```bash
ls /keys
```

Delete a key:
```bash
rm /keys/<key>
```

Rename a key:
```bash
mv /keys/<oldkey> /keys/<newkey>
```

## Examples

```bash
# Set a value
agfs:/> echo "hello world" > /kvfs/keys/mykey

# Get a value
agfs:/> cat /kvfs/keys/mykey
hello world

# List all keys
agfs:/> ls /kvfs/keys

# Delete a key
agfs:/> rm /kvfs/keys/mykey

# Rename a key
agfs:/> mv /kvfs/keys/oldname /kvfs/keys/newname
```

## License

Apache License 2.0
