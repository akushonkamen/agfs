# QueueFS Plugin - Message Queue Service

This plugin provides a message queue service through a file system interface.

## Files
```bash
/enqueue  # Write-only file to enqueue messages
/dequeue  # Read-only file to dequeue messages
/peek     # Read-only file to peek at next message
/size     # Read-only file showing queue size
/clear    # Write-only file to clear all messages
/README   # This file
```

## Dynamic Mounting With AGFS Shell

Interactive shell:
```bash  
agfs:/> mount queuefs /queue
agfs:/> mount queuefs /tasks
agfs:/> mount queuefs /messages
```
Direct command:
```bash
uv run agfs mount queuefs /queue
uv run agfs mount queuefs /jobs
```

## Configuration Parameters

None required - QueueFS works with default settings

## Usage
Enqueue a message:
```bash
echo "your message" > /enqueue
```

Dequeue a message:
```bash    
cat /dequeue
```

Peek at next message (without removing):
```bash    
cat /peek
```

Get queue size:
```bash    
cat /size
```

Clear the queue:
```bash
echo "" > /clear
```

## Example
```bash
# Enqueue a message  
agfs:/> echo "task-123" > /queuefs/enqueue

# Check queue size
agfs:/> cat /queuefs/size
1
# Dequeue a message
agfs:/> cat /queuefs/dequeue
{"id":"...","data":"task-123","timestamp":"..."}
```

## License

Apache License 2.0
