# <img src="./assets/logo-white.png" alt="CtxFS Logo" height="40" style="vertical-align: middle;"/>

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

**Aggregated File System (Agent FS)** - Everything is a file, in RESTful APIs. A tribute to Plan9.

> **Note**: CtxFS has been rewritten in [Rust](https://www.rust-lang.org/) for better performance, memory safety, and maintainability. The API remains fully compatible with the original Go implementation.

## Why CtxFS?

When coordinating multiple AI Agents in a distributed environment, agents need access to various backend services: message queues, databases, object storage, KV stores, and more. The traditional approach requires writing specialized API calls for each service, meaning agents must understand many different interfaces.

The core idea of CtxFS is simple: **unify all services as file system operations**.

```
Traditional approach                    CtxFS approach
------------------------------------------------------------------
redis.set("key", "value")          ->   echo "value" > /kvfs/keys/mykey
sqs.send_message(queue, msg)       ->   echo "msg" > /queuefs/q/enqueue
s3.put_object(bucket, key, data)   ->   cp file /s3fs/bucket/key
mysql.execute("SELECT ...")        ->   echo "SELECT ..." > /sqlfs2/.../query
```

The benefits:

1. **AI understands file operations natively** - Any LLM knows how to use cat, echo, and ls. No API documentation needed.
2. **Unified interface** - Operate all backends the same way, reducing cognitive overhead.
3. **Composability** - Combine services using pipes, redirections, and other shell features.
4. **Easy debugging** - Use ls and cat to inspect system state.

## Performance

Compared to the original Go implementation, the Rust version offers:

| Metric | Go CtxFS | Rust CtxFS | Improvement |
|--------|---------|-----------|-------------|
| Binary Size | 33MB | 8MB | **76% smaller** |
| Memory Usage | 15MB | 7MB | **54% less** |
| Large File Write (10MB) | 390ms | 183ms | **2.1x faster** |
| Large File Read (10MB) | 27ms | 9ms | **3x faster** |

See [BENCHMARK_REPORT.md](./BENCHMARK_REPORT.md) for detailed performance analysis.

## Quick Start

### Build from source

```bash
# Clone the repository
git clone https://github.com/c4pt0r/agfs.git
cd agfs

# Build the server
cd src
cargo build --release

# The binaries will be in:
# - target/release/agfs-server
# - target/release/agfs-fuse
```

### Run the server

```bash
# Using the built binary
./target/release/agfs-server --config config.yaml

# Or using cargo
cargo run --release --bin agfs-server -- -c config.yaml
```

Example configuration (`config.yaml`):

```yaml
server:
  address: ":8080"
  log_level: info

plugins:
  memfs:
    enabled: true
    path: /
    config: {}

  localfs:
    enabled: true
    path: /local
    config:
      local_dir: /tmp/agfs-local
```

### Docker

```bash
# Build the Docker image
docker build -t agfs-rust:latest .

# Run the server (HTTP API only)
docker run -p 8080:8080 agfs-rust:latest
```

### Connect using agfs-shell

```bash
$ agfs
agfs:/> ls
queuefs/  kvfs/  s3fs/  sqlfs/  heartbeatfs/  memfs/  ...
```

## FUSE Support

CtxFS can be mounted as a native filesystem on Linux using FUSE. This allows any program to interact with CtxFS services using standard file operations, not just the agfs-shell.

```bash
# Mount CtxFS to /mnt/agfs
agfs-fuse --agfs-server-url http://localhost:8080 --mount /mnt/agfs

# Now use standard tools
ls /mnt/agfs/kvfs/keys/
echo "hello" > /mnt/agfs/kvfs/keys/mykey
cat /mnt/agfs/queuefs/tasks/dequeue
```

This makes CtxFS accessible to any application, script, or programming language that can read and write files.

## Examples

### Key-Value Store

The simplest key-value storage. Filename is the key, content is the value:

```bash
agfs:/> echo "world" > /kvfs/keys/hello      # write
agfs:/> cat /kvfs/keys/hello                  # read -> "world"
agfs:/> ls /kvfs/keys/                        # list all keys
hello
agfs:/> rm /kvfs/keys/hello                   # delete
```

### Message Queue

A message queue is abstracted as a directory containing control files:

```bash
agfs:/> mkdir /queuefs/tasks             # create queue
agfs:/> ls /queuefs/tasks
enqueue  dequeue  peek  size  clear

agfs:/> echo "job1" > /queuefs/tasks/enqueue    # enqueue
019aa869-1a20-7ca6-a77a-b081e24c0593

agfs:/> cat /queuefs/tasks/size                 # check queue length
1

agfs:/> cat /queuefs/tasks/dequeue              # dequeue
{"id":"019aa869-...","data":"job1","timestamp":"2025-11-21T13:54:11Z"}
```

This pattern is ideal for AI Agent task distribution: one agent writes tasks to the queue, another agent reads and executes them.

### SQL Database

Query databases through a Plan 9 style session interface:

```bash
agfs:/> cat /sqlfs2/mydb/users/schema       # view table structure
agfs:/> cat /sqlfs2/mydb/users/count        # get row count

# Create session, execute query, read result
agfs:/> sid=$(cat /sqlfs2/mydb/users/ctl)
agfs:/> echo "SELECT * FROM users LIMIT 2" > /sqlfs2/mydb/users/$sid/query
agfs:/> cat /sqlfs2/mydb/users/$sid/result
[{"id": 1, "name": "alice"}, {"id": 2, "name": "bob"}]
```

### Agent Heartbeat

Manage the liveness state of distributed agents:

```bash
agfs:/> mkdir /heartbeatfs/agent-1       # register agent
agfs:/> touch /heartbeatfs/agent-1/keepalive   # send heartbeat

agfs:/> cat /heartbeatfs/agent-1/ctl     # check status
last_heartbeat_ts: 2025-11-21T13:55:45-08:00
timeout: 30
status: alive

# After 30 seconds without a new heartbeat, the agent directory is automatically removed
```

### Cross-FS Operations

Different filesystems can operate with each other:

```bash
agfs:/> cp local:/tmp/data.txt /s3fs/mybucket/   # upload local file to S3
agfs:/> cp /s3fs/mybucket/config.json /memfs/    # copy S3 file to memory
```

## CtxFS Scripts

CtxFS shell supports scripting with `.as` files. Scripts use familiar shell syntax and can be executed directly.

**task_worker.as** - A simple task queue worker:

```bash
#!/usr/bin/env agfs

QUEUE_PATH=/queuefs/tasks
POLL_INTERVAL=2

# Initialize queue
mkdir $QUEUE_PATH

while true; do
    size=$(cat $QUEUE_PATH/size)

    if [ "$size" = "0" ]; then
        echo "Queue empty, waiting..."
        sleep $POLL_INTERVAL
        continue
    fi

    # Dequeue and process task
    task=$(cat $QUEUE_PATH/dequeue)
    echo "Processing: $task"

    # Your task logic here
done
```

**enqueue_task.as** - Enqueue a task:

```bash
#!/usr/bin/env agfs

mkdir /queuefs/tasks
echo "$1" > /queuefs/tasks/enqueue
echo "Task enqueued. Queue size: $(cat /queuefs/tasks/size)"
```

Run scripts directly:

```bash
./task_worker.as &
./enqueue_task.as "process report.pdf"
```

See more examples in [src/agfs-shell/examples](./src/agfs-shell/examples/).

## Use Case: AI Agent Task Loop

A typical agent coordination pattern: multiple agents fetch tasks from the same queue and execute them.

```python
while True:
    task = agfs.cat("/queuefs/tasks/dequeue")
    if task:
        result = execute_task(task)
        agfs.write(f"/kvfs/keys/result_{task.id}", result)
```

See [task_loop.py](./src/agfs-mcp/demos/task_loop.py) for a complete example.

## Documentation

- [src/agfs-server](./src/agfs-server/) - Server implementation (Rust)
- [src/agfs-sdk](./src/agfs-sdk/) - SDK and type definitions (Rust)
- [src/agfs-fuse](./src/agfs-fuse/) - FUSE filesystem mount (Rust, Linux)
- [src/agfs-shell](./src/agfs-shell/) - Interactive shell client (Python)
- [src/agfs-mcp](./src/agfs-mcp/) - MCP integration (Python)
- [src/python-sdk](./src/python-sdk/) - Python SDK

## Architecture

CtxFS is built with a plugin architecture:

```
agfs-server (Rust)
├── HTTP Server (Axum)
├── MountableFS (Radix Tree Router)
└── Plugins
    ├── memfs - In-memory filesystem
    ├── localfs - Local directory mount
    ├── kvfs - Key-value storage
    ├── queuefs - Message queue (SQLite)
    ├── s3fs - S3 object storage
    ├── sqlfs/sqlfs2 - SQL database interface
    ├── streamfs - Stream multiplexing
    ├── heartbeatfs - Agent heartbeat tracking
    ├── httpfs - HTTP request proxy
    ├── proxyfs - CtxFS-to-CtxFS proxy
    └── ... (more plugins)
```

## License

Apache License 2.0
