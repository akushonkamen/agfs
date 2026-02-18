# HeartbeatFS Plugin - Heartbeat Monitoring Service

This plugin provides a heartbeat monitoring service through a file system interface.

## Structure
```bash
/<name>/           - Directory for each heartbeat item (auto-deleted when expired)
/<name>/keepalive  - Touch or write to update heartbeat
/<name>/ctl        - Read to get status, write to update timeout (timeout=N in seconds)
/README            - This file
```

# Usage
Create a new heartbeat item:
```bash
mkdir /heartbeatfs/<name>
```

Update heartbeat (keepalive):
```bash
touch /heartbeatfs/<name>/keepalive
echo "ping" > /heartbeatfs/<name>/keepalive
```

Update timeout:
```bash
echo "timeout=60" > /heartbeatfs/<name>/ctl
```

Check heartbeat status:
```bash
cat /heartbeatfs/<name>/ctl
```

Check if heartbeat is alive (stat will fail if expired):
```bash
stat /heartbeatfs/<name>
```

List all heartbeat items:
```bash
ls /heartbeatfs
```

Remove heartbeat item:
```bash
rm -r /heartbeatfs/<name>
```

## Behavior
- Default timeout: 5 minutes (300 seconds) from last heartbeat
- Timeout can be customized per item by writing to ctl file
- Expired items are automatically removed by the system
- Use stat to check if an item still exists (alive)

## Examples
```bash
# Create a heartbeat item
agfs:/> mkdir /heartbeatfs/myservice

# Send heartbeat
agfs:/> touch /heartbeatfs/myservice/keepalive

# Set custom timeout (60 seconds)
agfs:/> echo "timeout=60" > /heartbeatfs/myservice/ctl

# Check status
agfs:/> cat /heartbeatfs/myservice/ctl
last_heartbeat_ts: 2024-11-21T10:30:00Z
expire_ts: 2024-11-21T10:31:00Z
timeout: 60
status: alive

# Check if still alive (will fail if expired)
agfs:/> stat /heartbeatfs/myservice
```

## License

Apache License 2.0
