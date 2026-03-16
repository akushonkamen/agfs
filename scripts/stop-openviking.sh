#!/bin/bash
# OpenViking + AGFS Rust 停止脚本

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

OV_DIR="/home/yalun/Dev/OpenViking"
OV_PID_FILE="$OV_DIR/openviking-server.pid"

echo -e "${YELLOW}停止 OpenViking + AGFS...${NC}"
echo ""

# 1. 停止 OpenViking 服务器
echo "=== 停止 OpenViking 服务器 ==="
if pgrep -f "openviking-server" > /dev/null; then
    pkill -f "openviking-server"
    sleep 2
    if ! pgrep -f "openviking-server" > /dev/null; then
        echo -e "${GREEN}✓ OpenViking 服务器已停止${NC}"
    else
        echo -e "${RED}强制停止 OpenViking 服务器...${NC}"
        pkill -9 -f "openviking-server"
    fi
else
    echo -e "${YELLOW}OpenViking 服务器未运行${NC}"
fi

rm -f "$OV_PID_FILE" 2>/dev/null || true

# 2. 停止 AGFS 服务器（可选）
echo ""
echo "=== 停止 AGFS 服务器 ==="
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if pgrep -f "agfs-server" > /dev/null; then
    cd "$SCRIPT_DIR"
    ./stop-agfs.sh
else
    echo -e "${YELLOW}AGFS 服务器未运行${NC}"
fi

echo ""
echo -e "${GREEN}=== 所有服务已停止 ===${NC}"
