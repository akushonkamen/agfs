#!/bin/bash
# CtxFS 性能基准测试
# 对比不同操作的吞吐量和延迟

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[BENCH]${NC} $1"; }
log_error() { echo -e "${RED}[FAIL]${NC} $1"; }

# 配置
SERVER_URL="${CTXFS_SERVER_URL:-http://localhost:8080/api/v1}"
FIXTURES_DIR="$SCRIPT_DIR/fixtures"

# ========== 测试辅助函数 ==========

# 生成随机数据
gen_data() {
  local size=$1
  dd if=/dev/urandom bs=$size count=1 2>/dev/null
}

# 获取毫秒时间戳
ms_now() {
  date +%s%3N
}

# ========== 性能测试 ==========

echo ""
echo "=========================================="
echo "     CtxFS 性能基准测试"
echo "=========================================="
echo ""

# 检查服务是否运行
log_info "检查服务状态..."
if ! curl -s "${SERVER_URL%/api/v1}/health" >/dev/null 2>&1; then
  log_error "CtxFS 服务未运行，请先启动服务"
  echo "  启动: ./start-agfs.sh"
  exit 1
fi
echo "  服务: $SERVER_URL"
echo ""

# ========== 1. 小文件写入测试 ==========
log_info "1. 小文件写入测试 (100 个 1KB 文件)..."
START=$(ms_now)
for i in $(seq 1 100); do
  curl -s -X POST "${SERVER_URL}/files?path=/bench-small-$i.txt" \
    -d "$(gen_data 1024)" >/dev/null
done
ELAPSED=$(($(ms_now) - START))
RATE=$((100000 / ELAPSED))
echo "  耗时: ${ELAPSED}ms"
echo "  吞吐: ${RATE} ops/sec"
echo "  延迟: $((ELAPSED / 100)) ms/文件"
echo ""

# ========== 2. 小文件读取测试 ==========
log_info "2. 小文件读取测试 (100 个文件)..."
START=$(ms_now)
for i in $(seq 1 100); do
  curl -s "${SERVER_URL}/files?path=/bench-small-$i.txt" >/dev/null
done
ELAPSED=$(($(ms_now) - START))
RATE=$((100000 / ELAPSED))
echo "  耗时: ${ELAPSED}ms"
echo "  吞吐: ${RATE} ops/sec"
echo "  延迟: $((ELAPSED / 100)) ms/文件"
echo ""

# ========== 3. 并发写入测试 ==========
log_info "3. 并发写入测试 (50 个并发请求)..."
START=$(ms_now)
for i in $(seq 1 50); do
  curl -s -X POST "${SERVER_URL}/files?path=/bench-concurrent-$i.txt" \
    -d "data-$i" &
done
wait
ELAPSED=$(($(ms_now) - START))
RATE=$((50000 / ELAPSED))
echo "  耗时: ${ELAPSED}ms"
echo "  吞吐: ${RATE} ops/sec"
echo ""

# ========== 4. 目录列表测试 ==========
log_info "4. 目录列表测试 (1000 个文件)..."
# 先创建文件
for i in $(seq 1 1000); do
  curl -s -X POST "${SERVER_URL}/files?path=/bench-dir/file-$i.txt" \
    -d "data" >/dev/null
done
START=$(ms_now)
curl -s "${SERVER_URL}/directories?path=/" >/dev/null
ELAPSED=$(($(ms_now) - START))
echo "  耗时: ${ELAPSED}ms"
echo "  速率: 快速"
echo ""

# ========== 5. 大文件测试 ==========
log_info "5. 大文件写入测试 (1MB)..."
DATA_FILE="$FIXTURES_DIR/1mb.bin"
if [ ! -f "$DATA_FILE" ]; then
  mkdir -p "$FIXTURES_DIR"
  dd if=/dev/urandom of="$DATA_FILE" bs=1048576 count=1 2>/dev/null
fi

START=$(ms_now)
curl -s -X PUT "${SERVER_URL}/files?path=/bench-large.bin" \
  --data-binary "@$DATA_FILE" >/dev/null
ELAPSED=$(($(ms_now) - START))
THROUGHPUT=$((1048576 * 1000 / ELAPSED / 1024 / 1024))
echo "  耗时: ${ELAPSED}ms"
echo "  吞吐: ${THROUGHPUT} MB/sec"
echo ""

# ========== 6. 大文件读取测试 ==========
log_info "6. 大文件读取测试 (1MB)..."
START=$(ms_now)
curl -s "${SERVER_URL}/files?path=/bench-large.bin" -o /dev/null
ELAPSED=$(($(ms_now) - START))
THROUGHPUT=$((1048576 * 1000 / ELAPSED / 1024 / 1024))
echo "  耗时: ${ELAPSED}ms"
echo "  吞吐: ${THROUGHPUT} MB/sec"
echo ""

# ========== 7. 操作延迟测试 ==========
log_info "7. 各操作延迟测试..."
for op in "stat" "touch" "remove"; do
  START=$(ms_now)
  for i in $(seq 1 100); do
    case $op in
      stat)
        curl -s "${SERVER_URL}/stat?path=/bench-small-1.txt" >/dev/null
        ;;
      touch)
        curl -s -X POST "${SERVER_URL}/touch?path=/bench-touch-$i.txt" >/dev/null
        ;;
      remove)
        curl -s -X POST "${SERVER_URL}/files/delete?path=/bench-concurrent-$i.txt" >/dev/null
        ;;
    esac
  done
  ELAPSED=$(($(ms_now) - START))
  echo "  $op: $((ELAPSED / 100)) ms/op ($((100000 / ELAPSED)) ops/sec)"
done
echo ""

# ========== 8. 资源占用 ==========
log_info "8. 资源占用..."
SERVER_PID=$(pgrep -f "ctxfs-server" | head -1)
if [ -n "$SERVER_PID" ]; then
  if [ -f "/proc/$SERVER_PID/status" ]; then
    MEM_KB=$(grep VmRSS /proc/$SERVER_PID/status | awk '{print $2}')
    echo "  内存: $((MEM_KB / 1024)) MB (RSS)"
  fi
  THREADS=$(ls /proc/$SERVER_PID/task 2>/dev/null | wc -l)
  echo "  线程: $THREADS"
  FDS=$(ls /proc/$SERVER_PID/fd 2>/dev/null | wc -l)
  echo "  文件描述符: $FDS"

  BINARY_SIZE=$(stat -c%s "$PROJECT_ROOT/src/target/debug/ctxfs-server" 2>/dev/null || echo "0")
  if [ "$BINARY_SIZE" != "0" ]; then
    echo "  二进制大小: $((BINARY_SIZE / 1024 / 1024)) MB (debug)"
  fi
fi
echo ""

# ========== 清理 ==========
log_info "清理测试数据..."
for i in $(seq 1 100); do
  curl -s -X POST "${SERVER_URL}/files/delete?path=/bench-small-$i.txt" >/dev/null
done
for i in $(seq 1 1000); do
  curl -s -X POST "${SERVER_URL}/files/delete?path=/bench-dir/file-$i.txt" >/dev/null
done
curl -s -X POST "${SERVER_URL}/files/delete?path=/bench-large.bin" >/dev/null
for i in $(seq 1 100); do
  curl -s -X POST "${SERVER_URL}/files/delete?path=/bench-touch-$i.txt" >/dev/null
done
echo "  清理完成"
echo ""

echo "=========================================="
echo "  性能测试完成"
echo "=========================================="
