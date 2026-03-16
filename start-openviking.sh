#!/bin/bash
# OpenViking + AGFS Rust 启动脚本

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export PATH=$HOME/.local/bin:$PATH

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# OpenViking 目录
OV_DIR="/home/yalun/Dev/OpenViking"
OV_DATA_DIR="$OV_DIR/data"
OV_LOG_DIR="$OV_DIR/log"
OV_PID_FILE="$OV_DIR/openviking-server.pid"

echo -e "${GREEN}启动 OpenViking + AGFS Rust...${NC}"

# 1. 确保 AGFS Rust 服务器正在运行
echo ""
echo "=== 检查 AGFS Rust 服务器 ==="
if pgrep -f "agfs-server" > /dev/null; then
    echo -e "${GREEN}✓ AGFS 服务器已运行${NC}"
else
    echo "启动 AGFS 服务器..."
    cd "$SCRIPT_DIR"
    ./start-agfs.sh
fi

# 2. 确保 OpenViking 目录存在
echo ""
echo "=== 准备 OpenViking 环境 ==="
mkdir -p "$OV_DATA_DIR"
mkdir -p "$OV_LOG_DIR"
echo "数据目录: $OV_DATA_DIR"
echo "日志目录: $OV_LOG_DIR"

# 3. 检查是否已有 OpenViking 在运行
echo ""
echo "=== 检查 OpenViking 服务器 ==="
if [ -f "$OV_PID_FILE" ]; then
    PID=$(cat "$OV_PID_FILE")
    if ps -p "$PID" > /dev/null 2>&1; then
        echo -e "${YELLOW}OpenViking 服务器已在运行 (PID: $PID)${NC}"
    else
        rm -f "$OV_PID_FILE"
    fi
fi

if ! pgrep -f "openviking-server" > /dev/null; then
    echo "启动 OpenViking 服务器..."
    cd "$OV_DIR"
    nohup openviking-server > "$OV_LOG_DIR/server.log" 2>&1 &
    echo $! > "$OV_PID_FILE"

    # 等待服务器启动
    sleep 3

    if pgrep -f "openviking-server" > /dev/null; then
        echo -e "${GREEN}✓ OpenViking 服务器启动成功!${NC}"
    else
        echo -e "${RED}✗ OpenViking 服务器启动失败!${NC}"
        echo "查看日志: cat $OV_LOG_DIR/server.log"
        exit 1
    fi
else
    echo -e "${GREEN}✓ OpenViking 服务器已运行${NC}"
fi

# 4. 健康检查
echo ""
echo "=== 健康检查 ==="

# AGFS
if curl -s http://localhost:8080/api/v1/health > /dev/null; then
    echo -e "${GREEN}✓ AGFS (8080): 健康${NC}"
else
    echo -e "${RED}✗ AGFS (8080): 不健康${NC}"
fi

# OpenViking
if curl -s http://localhost:8888/api/v1/observer/system > /dev/null; then
    echo -e "${GREEN}✓ OpenViking (8888): 健康${NC}"
else
    echo -e "${RED}✗ OpenViking (8888): 不健康${NC}"
fi

# 5. 使用提示
echo ""
echo -e "${GREEN}=== 服务已启动 ===${NC}"
echo ""
echo "OpenViking CLI 命令:"
echo "  openviking health              # 健康检查"
echo "  openviking add-resource <url>  # 添加资源"
echo "  openviking ls viking://resources/  # 列出资源"
echo "  openviking find \"查询内容\"     # 搜索"
echo ""
echo "停止服务:"
echo "  $SCRIPT_DIR/stop-openviking.sh"
echo ""
