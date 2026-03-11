# ServerInfoFS Plugin - Server Metadata and Information

This plugin provides runtime information about the AGFS server.

## Mount
```bash  
agfs:/> mount serverinfofs /info
```

## Usage
View server version:
```bash
cat /version
```

View server uptime:
```bash
cat /uptime
```

View server info:
```bash
cat /info
```

## Files
```bash  
/version  - Server version information
/uptime   - Server uptime since start
/info     - Complete server information (JSON)
/README   - This file
```

## Examples
```bash
# Check server version
agfs:/> cat /serverinfofs/version
1.0.0

# Check uptime
agfs:/> cat /serverinfofs/uptime
Server uptime: 5m30s

# Get complete info
agfs:/> cat /serverinfofs/server_info
{
"version": "1.0.0",
"uptime": "5m30s",
"go_version": "go1.21",
...
}
```

## License

Apache License 2.0
