#!/usr/bin/env bash
# =============================================================================
# Go→Rust 重构团队工作区初始化脚本
# 用法: ./scripts/init_workspace.sh <go-project-path> <rust-project-name>
# =============================================================================

set -euo pipefail

GO_SRC="${1:-./go-src}"
RUST_NAME="${2:-rust-rewrite}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# ── 颜色输出 ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
info()    { echo -e "${BLUE}[INFO]${NC}  $*"; }
success() { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[ERR]${NC}   $*"; exit 1; }

# ── 检查前置条件 ───────────────────────────────────────────────────────────────
check_prereqs() {
  info "检查前置条件..."
  command -v claude >/dev/null 2>&1 || error "Claude Code 未安装，请先安装：npm install -g @anthropic-ai/claude-code"
  
  local version
  version=$(claude --version 2>/dev/null | grep -oP '\d+\.\d+\.\d+' | head -1 || echo "0.0.0")
  # Agent Teams requires v2.1.32+
  if [[ "$(printf '%s\n' "2.1.32" "$version" | sort -V | head -1)" != "2.1.32" ]]; then
    error "Claude Code 版本 $version 不支持 Agent Teams，需要 >= 2.1.32。请更新：npm update -g @anthropic-ai/claude-code"
  fi
  success "Claude Code $version ✓"
  
  command -v cargo >/dev/null 2>&1 || error "Rust/Cargo 未安装，请访问 https://rustup.rs"
  success "Cargo $(cargo --version) ✓"
  
  if command -v tmux >/dev/null 2>&1; then
    success "tmux $(tmux -V) ✓ (split-pane 模式可用)"
  else
    warn "tmux 未安装，将使用 in-process 模式（Shift+Down 切换 Teammate）"
    warn "建议安装: brew install tmux 或 apt install tmux"
  fi
}

# ── 创建目录结构 ───────────────────────────────────────────────────────────────
create_structure() {
  info "创建团队工作区目录结构..."
  
  mkdir -p "$ROOT_DIR"/{.rust-rewrite/{team,reports,phases},.claude,rust-src,scripts}
  
  # 如果 go-src 不是符号链接且路径存在，创建符号链接
  if [[ -d "$GO_SRC" && "$GO_SRC" != "$ROOT_DIR/go-src" ]]; then
    ln -sfn "$(realpath "$GO_SRC")" "$ROOT_DIR/go-src"
    success "已链接 Go 源码：$GO_SRC → $ROOT_DIR/go-src"
  elif [[ ! -d "$ROOT_DIR/go-src" ]]; then
    warn "Go 源码目录不存在: $GO_SRC"
    mkdir -p "$ROOT_DIR/go-src"
    warn "已创建空目录 go-src/，请手动放入 Go 源码"
  fi
  
  success "目录结构创建完成"
}

# ── 复制模板文件 ───────────────────────────────────────────────────────────────
copy_templates() {
  info "部署模板文件..."
  
  local TEMPLATE_SRC="$SCRIPT_DIR/../.rust-rewrite/team/template"
  
  # 保留模板目录
  [[ -d "$TEMPLATE_SRC" ]] || { warn "模板目录不存在，跳过"; return; }
  
  success "模板就绪：$TEMPLATE_SRC"
}

# ── 初始化 Rust 项目 ───────────────────────────────────────────────────────────
init_rust_project() {
  info "初始化 Rust 项目..."
  
  local RUST_DIR="$ROOT_DIR/rust-src"
  
  if [[ -f "$RUST_DIR/Cargo.toml" ]]; then
    warn "Rust 项目已存在，跳过初始化"
    return
  fi
  
  cd "$RUST_DIR"
  cargo init --name "$RUST_NAME" 2>/dev/null || cargo init
  
  # 创建基础测试目录
  mkdir -p tests/{unit,integration}
  touch tests/integration/mod.rs
  
  success "Rust 项目初始化完成：$RUST_NAME"
  cd "$ROOT_DIR"
}

# ── 更新 settings.json ─────────────────────────────────────────────────────────
configure_settings() {
  info "配置 Claude Code Agent Teams..."
  
  local SETTINGS="$ROOT_DIR/.claude/settings.json"
  cat > "$SETTINGS" <<'EOF'
{
  "env": {
    "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"
  },
  "model": "claude-opus-4-6",
  "preferredNotifChannel": "terminal_bell"
}
EOF
  success "Agent Teams 已启用：$SETTINGS"
}

# ── 分析 Go 项目结构 ───────────────────────────────────────────────────────────
analyze_go_project() {
  info "分析 Go 项目结构..."
  
  local GO_DIR="$ROOT_DIR/go-src"
  local ANALYSIS="$ROOT_DIR/.rust-rewrite/GO_ANALYSIS.md"
  
  if [[ ! -d "$GO_DIR" ]] || [[ -z "$(ls -A "$GO_DIR" 2>/dev/null)" ]]; then
    warn "go-src/ 目录为空，跳过分析"
    return
  fi
  
  {
    echo "# Go 项目结构分析"
    echo "> 生成时间：$(date '+%Y-%m-%d %H:%M')"
    echo ""
    echo "## 目录结构"
    echo '```'
    find "$GO_DIR" -type f -name "*.go" | sort | head -100
    echo '```'
    echo ""
    echo "## Go 模块信息"
    if [[ -f "$GO_DIR/go.mod" ]]; then
      echo '```'
      cat "$GO_DIR/go.mod"
      echo '```'
    fi
    echo ""
    echo "## 包结构（top-level packages）"
    find "$GO_DIR" -type d | grep -v vendor | grep -v ".git" | sort | head -50
    echo ""
    echo "## 测试文件"
    echo '```'
    find "$GO_DIR" -name "*_test.go" | sort
    echo '```'
  } > "$ANALYSIS"
  
  success "Go 项目分析报告已生成：$ANALYSIS"
}

# ── 打印使用说明 ───────────────────────────────────────────────────────────────
print_instructions() {
  echo ""
  echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
  echo -e "${GREEN}  ✅ Go→Rust 重构团队工作区初始化完成！${NC}"
  echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
  echo ""
  echo "📁 工作区结构："
  echo "   $ROOT_DIR/"
  echo "   ├── CLAUDE.md              ← 团队共享规范（所有 Agent 自动读取）"
  echo "   ├── LEADER_PROMPT.md       ← Leader 初始 Prompt"
  echo "   ├── go-src/                ← Go 原始项目（只读）"
  echo "   ├── rust-src/              ← Rust 重写目标"
  echo "   ├── .claude/settings.json  ← Agent Teams 已启用"
  echo "   └── .rust-rewrite/         ← 团队协作工作区"
  echo "       ├── PHASES.md          ← Phase 计划追踪"
  echo "       ├── TEAM_ROSTER.md     ← 成员清单"
  echo "       └── team/template/     ← discussion.md/task.md 模板"
  echo ""
  echo "🚀 启动步骤："
  echo ""
  echo "  1. 进入项目目录："
  echo "     cd $ROOT_DIR"
  echo ""
  if command -v tmux >/dev/null 2>&1; then
    echo "  2. 启动 tmux 会话（推荐，可同时观察所有 Teammate）："
    echo "     tmux new-session -s go2rust"
    echo ""
    echo "  3. 启动 Claude Code（Leader）："
    echo "     claude"
  else
    echo "  2. 启动 Claude Code（Leader）："
    echo "     claude"
  fi
  echo ""
  echo "  3. 给 Leader 发送以下启动 Prompt："
  echo ""
  echo -e "     ${YELLOW}┌─────────────────────────────────────────────────────┐${NC}"
  echo -e "     ${YELLOW}│ 读取 LEADER_PROMPT.md 中的完整指令并开始工作。     │${NC}"
  echo -e "     ${YELLOW}│ 这是一个 Go→Rust 重构项目，请按照文件中的指引，   │${NC}"
  echo -e "     ${YELLOW}│ 首先分析 go-src/ 目录，然后制定 Phase 计划，       │${NC}"
  echo -e "     ${YELLOW}│ 最后创建 Agent Team 开始重构工作。                 │${NC}"
  echo -e "     ${YELLOW}└─────────────────────────────────────────────────────┘${NC}"
  echo ""
  echo "  4. 切换 Teammate 视图："
  echo "     - In-process 模式：按 Shift+Down 循环切换"
  if command -v tmux >/dev/null 2>&1; then
    echo "     - Split-pane 模式：tmux 自动在新 pane 中显示每个 Teammate"
  fi
  echo ""
  echo "📖 关键文件说明："
  echo "   - LEADER_PROMPT.md：完整的 Leader 工作指引，发给 Leader 即可"
  echo "   - CLAUDE.md：团队规范，所有 Agent 启动时自动加载"
  echo "   - .rust-rewrite/team/template/：创建新 Teammate 工作区的模板"
  echo ""
}

# ── 主函数 ────────────────────────────────────────────────────────────────────
main() {
  echo ""
  echo -e "${BLUE}🦀 Go→Rust 重构团队 · 工作区初始化${NC}"
  echo ""
  
  check_prereqs
  create_structure
  copy_templates
  init_rust_project
  configure_settings
  analyze_go_project
  print_instructions
}

main "$@"
