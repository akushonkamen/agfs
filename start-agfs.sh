#!/bin/bash
# AGFS Rust Server启动脚本
# 用于 OpenViking 集成

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 配置
SERVER_BIN="${SCRIPT_DIR}/rust-src/target/release/agfs-server"
CONFIG_FILE="${SCRIPT_DIR}/test-config.yaml"
PID_FILE="/tmp/agfs-server.pid"
LOG_FILE="/tmp/agfs-server.log"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查二进制文件
if [ ! -f "$SERVER_BIN" ]; then
    echo -e "${RED}错误: agfs-server 二进制文件不存在${NC}"
    echo "请先运行: cd rust-src && cargo build --release"
    exit 1
fi

# 检查配置文件
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${RED}错误: 配置文件不存在: $CONFIG_FILE${NC}"
    exit 1
fi

# 检查是否已在运行
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if ps -p "$PID" > /dev/null 2>&1; then
        echo -e "${YELLOW}agfs-server 已在运行 (PID: $PID)${NC}"
        echo "如需重启，请先运行: $0 stop"
        exit 0
    else
        rm -f "$PID_FILE"
    fi
fi

# 确保本地存储目录存在
mkdir -p /tmp/agfs-local

# 启动服务器
echo -e "${GREEN}启动 AGFS Rust Server...${NC}"
echo "  二进制: $SERVER_BIN"
echo "  配置: $CONFIG_FILE"
echo "  日志: $LOG_FILE"

nohup "$SERVER_BIN" --config "$CONFIG_FILE" > "$LOG_FILE" 2>&1 &
echo $! > "$PID_FILE"

# 等待服务启动
sleep 2

# 检查服务是否正常启动
if ps -p $(cat "$PID_FILE") > /dev/null 2>&1; then
    echo -e "${GREEN}✓ AGFS Server 启动成功!${NC}"
    echo "  PID: $(cat $PID_FILE)"
    echo "  端口: 8080"
    echo ""
    echo "测试连接:"
    echo "  curl http://localhost:8080/api/v1/health"
    echo ""
    echo "停止服务:"
    echo "  $0 stop"
else
    echo -e "${RED}✗ AGFS Server 启动失败!${NC}"
    echo "查看日志: cat $LOG_FILE"
    rm -f "$PID_FILE"
    exit 1
fi
