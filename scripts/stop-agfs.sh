#!/bin/bash
# CtxFS Rust Server停止脚本

set -e

PID_FILE="/tmp/ctxfs-server.pid"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

if [ ! -f "$PID_FILE" ]; then
    echo -e "${YELLOW}ctxfs-server 未在运行${NC}"
    exit 0
fi

PID=$(cat "$PID_FILE")

if ! ps -p "$PID" > /dev/null 2>&1; then
    echo -e "${YELLOW}ctxfs-server 进程不存在 (PID: $PID)${NC}"
    rm -f "$PID_FILE"
    exit 0
fi

echo -e "${YELLOW}停止 CtxFS Server (PID: $PID)...${NC}"
kill "$PID"

# 等待进程结束
for i in {1..10}; do
    if ! ps -p "$PID" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ CtxFS Server 已停止${NC}"
        rm -f "$PID_FILE"
        exit 0
    fi
    sleep 1
done

# 强制结束
echo -e "${RED}强制停止 CtxFS Server...${NC}"
kill -9 "$PID" 2>/dev/null || true
rm -f "$PID_FILE"
echo -e "${GREEN}✓ CtxFS Server 已强制停止${NC}"
