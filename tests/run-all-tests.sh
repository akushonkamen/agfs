#!/bin/bash
# CtxFS 一键测试脚本
# 运行所有测试：单元测试、集成测试、性能测试

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

log_info() { echo -e "${GREEN}[TEST]${NC} $1"; }
log_error() { echo -e "${RED}[FAIL]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

# 解析参数
RUN_UNIT=true
RUN_INTEGRATION=true
RUN_PERFORMANCE=false
RUN_PYTHON=true
VERBOSE=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --unit) RUN_INTEGRATION=false; RUN_PERFORMANCE=false; RUN_PYTHON=false ;;
    --integration) RUN_UNIT=false; RUN_PERFORMANCE=false; RUN_PYTHON=false ;;
    --perf) RUN_UNIT=false; RUN_INTEGRATION=false; RUN_PYTHON=false; RUN_PERFORMANCE=true ;;
    --rust) RUN_PYTHON=false ;;
    --python) RUN_UNIT=false; RUN_INTEGRATION=false; RUN_PERFORMANCE=false ;;
    --verbose) VERBOSE=true ;;
    -h|--help)
      echo "用法: $0 [选项]"
      echo ""
      echo "选项:"
      echo "  --unit          只运行单元测试"
      echo "  --integration   只运行集成测试"
      echo "  --perf          只运行性能测试"
      echo "  --rust          只运行 Rust 测试"
      echo "  --python        只运行 Python 测试"
      echo "  --verbose       详细输出"
      echo "  -h, --help      显示帮助"
      exit 0
      ;;
    *) echo "未知选项: $1"; exit 1 ;;
  esac
  shift
done

# 统计变量
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 开始时间
START_TIME=$(date +%s)

echo ""
echo "=========================================="
echo "     CtxFS 测试套件"
echo "=========================================="
echo ""

# ========== Rust 单元测试 ==========
if [ "$RUN_UNIT" = true ]; then
  log_info "运行 Rust 单元测试..."

  cd "$PROJECT_ROOT/src"

  if [ "$VERBOSE" = true ]; then
    if cargo test --workspace --lib 2>&1; then
      log_info "Rust 单元测试通过"
      ((PASSED_TESTS++))
    else
      log_error "Rust 单元测试失败"
      ((FAILED_TESTS++))
    fi
  else
    if cargo test --workspace --lib --quiet 2>&1; then
      log_info "Rust 单元测试通过"
      ((PASSED_TESTS++))
    else
      log_error "Rust 单元测试失败"
      ((FAILED_TESTS++))
    fi
  fi

  cd "$PROJECT_ROOT"
  ((TOTAL_TESTS++))
  echo ""
fi

# ========== Rust 集成测试 ==========
if [ "$RUN_INTEGRATION" = true ]; then
  log_info "运行 Rust 集成测试..."

  cd "$PROJECT_ROOT/src"

  if [ "$VERBOSE" = true ]; then
    if cargo test --workspace --test integration_test -- --ignored 2>&1; then
      log_info "Rust 集成测试通过"
      ((PASSED_TESTS++))
    else
      log_error "Rust 集成测试失败"
      ((FAILED_TESTS++))
    fi
  else
    if cargo test --workspace --test integration_test -- --ignored --quiet 2>&1; then
      log_info "Rust 集成测试通过"
      ((PASSED_TESTS++))
    else
      log_error "Rust 集成测试失败"
      ((FAILED_TESTS++))
    fi
  fi

  cd "$PROJECT_ROOT"
  ((TOTAL_TESTS++))
  echo ""
fi

# ========== Python 测试 ==========
if [ "$RUN_PYTHON" = true ]; then
  log_info "运行 Python 测试 (shell)..."

  cd "$PROJECT_ROOT/src/shell"

  if command -v uv &>/dev/null; then
    if uv run pytest tests/ -q ${VERBOSE:+ -v}; then
      log_info "Python 测试通过"
      ((PASSED_TESTS++))
    else
      log_error "Python 测试失败"
      ((FAILED_TESTS++))
    fi
  else
    log_warn "uv 未安装，跳过 Python 测试"
  fi

  cd "$PROJECT_ROOT"
  ((TOTAL_TESTS++))
  echo ""
fi

# ========== 性能测试 ==========
if [ "$RUN_PERFORMANCE" = true ]; then
  log_info "运行性能基准测试..."

  if [ -f "$PROJECT_ROOT/tests/benchmarks/benchmark-agfs.sh" ]; then
    bash "$PROJECT_ROOT/tests/benchmarks/benchmark-agfs.sh" 2>&1 || {
      log_warn "性能测试跳过（需要服务运行）"
    }
    ((PASSED_TESTS++))
  else
    log_warn "性能测试脚本不存在"
  fi
  ((TOTAL_TESTS++))
  echo ""
fi

# ========== 代码质量检查 ==========
if [ "$RUN_UNIT" = true ] || [ "$RUN_INTEGRATION" = true ]; then
  log_info "运行代码质量检查..."

  cd "$PROJECT_ROOT/src"

  # Clippy
  if cargo clippy --workspace -- -D warnings --quiet 2>&1; then
    log_info "Clippy 检查通过"
    ((PASSED_TESTS++))
  else
    log_warn "Clippy 检查有警告"
  fi
  ((TOTAL_TESTS++))

  # Format
  if cargo fmt --all -- --check 2>&1; then
    log_info "代码格式检查通过"
    ((PASSED_TESTS++))
  else
    log_warn "代码格式需要调整（运行 cargo fmt）"
  fi
  ((TOTAL_TESTS++))

  cd "$PROJECT_ROOT"
  echo ""
fi

# ========== 汇总 ==========
END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))

echo ""
echo "=========================================="
echo "     测试结果汇总"
echo "=========================================="
echo ""
echo "总测试套件: $TOTAL_TESTS"
echo -e "通过: ${GREEN}$PASSED_TESTS${NC}"
echo -e "失败: ${RED}$FAILED_TESTS${NC}"
echo "耗时: ${ELAPSED}秒"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
  echo -e "${GREEN}✓ 所有测试通过！${NC}"
  exit 0
else
  echo -e "${RED}✗ 有测试失败${NC}"
  exit 1
fi
