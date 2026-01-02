#!/bin/sh
set -e

# Default values
AGFS_SERVER_URL="${AGFS_SERVER_URL:-http://localhost:8080}"
AGFS_MOUNT_POINT="${AGFS_MOUNT_POINT:-/mnt/agfs}"
AGFS_CONFIG="${AGFS_CONFIG:-/config.yaml}"

echo "Starting AGFS Server..."
# Start agfs-server in background
/app/agfs-server -c "$AGFS_CONFIG" &
SERVER_PID=$!

# Wait for server to be ready
echo "Waiting for AGFS Server to be ready..."
max_retries=30
retry_count=0
while [ $retry_count -lt $max_retries ]; do
    if wget -q -O- "$AGFS_SERVER_URL/api/v1/health" >/dev/null 2>&1; then
        echo "AGFS Server is ready!"
        break
    fi
    retry_count=$((retry_count + 1))
    if [ $retry_count -eq $max_retries ]; then
        echo "ERROR: AGFS Server failed to start within timeout"
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
    echo "Waiting for server... ($retry_count/$max_retries)"
    sleep 1
done

# Create mount point if it doesn't exist
mkdir -p "$AGFS_MOUNT_POINT"

echo "Mounting AGFS to $AGFS_MOUNT_POINT..."
# Start agfs-fuse in foreground
exec agfs-fuse --agfs-server-url "$AGFS_SERVER_URL" --mount "$AGFS_MOUNT_POINT" --allow-other
