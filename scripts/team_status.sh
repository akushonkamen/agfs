#!/usr/bin/env bash
# =============================================================================
# 团队状态检查脚本（Leader 快速巡检）
# 用法: ./scripts/team_status.sh
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TEAM_DIR="$ROOT_DIR/.rust-rewrite/team"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'

echo ""
echo -e "${BOLD}🦀 Go→Rust 重构团队 · 状态总览${NC}"
echo -e "   $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# ── Phase 进度 ─────────────────────────────────────────────────────────────────
echo -e "${BOLD}📊 Phase 进度${NC}"
if [[ -f "$ROOT_DIR/.rust-rewrite/PHASES.md" ]]; then
  grep -E "^(## Phase|Phase [0-9])" "$ROOT_DIR/.rust-rewrite/PHASES.md" | head -20 || true
else
  echo -e "  ${YELLOW}PHASES.md 不存在${NC}"
fi
echo ""

# ── 各模块任务状态 ────────────────────────────────────────────────────────────
echo -e "${BOLD}📋 各模块任务状态${NC}"

if [[ ! -d "$TEAM_DIR" ]] || [[ -z "$(ls -A "$TEAM_DIR" 2>/dev/null)" ]]; then
  echo -e "  ${YELLOW}暂无 Teammate 工作区${NC}"
else
  for module_dir in "$TEAM_DIR"/*/; do
    [[ "$module_dir" == *"template"* ]] && continue
    module=$(basename "$module_dir")
    task_file="$module_dir/task.md"
    disc_file="$module_dir/discussion.md"
    
    echo -e "  ${BLUE}[$module]${NC}"
    
    if [[ -f "$task_file" ]]; then
      total=$(grep -c '^\- \[' "$task_file" 2>/dev/null || echo 0)
      done=$(grep -c '^\- \[x\]' "$task_file" 2>/dev/null || echo 0)
      blockers=$(grep -c '\[BLOCKER\]' "$task_file" 2>/dev/null || echo 0)
      
      if [[ $blockers -gt 0 ]]; then
        echo -e "    任务: $done/$total ✅  ${RED}⚠️  $blockers 个阻塞！${NC}"
      elif [[ $total -eq 0 ]]; then
        echo -e "    任务: ${YELLOW}任务未填写${NC}"
      elif [[ $done -eq $total ]]; then
        echo -e "    任务: ${GREEN}$done/$total ✅ 全部完成${NC}"
      else
        echo -e "    任务: $done/$total 进行中"
      fi
    else
      echo -e "    ${YELLOW}task.md 不存在${NC}"
    fi
    
    if [[ -f "$disc_file" ]]; then
      last_update=$(stat -c '%y' "$disc_file" 2>/dev/null | cut -d' ' -f1 || date '+%Y-%m-%d')
      echo -e "    discussion.md 最后更新: $last_update"
    fi
  done
fi
echo ""

# ── Rust 构建状态 ─────────────────────────────────────────────────────────────
echo -e "${BOLD}🔨 Rust 构建状态${NC}"
if [[ -f "$ROOT_DIR/rust-src/Cargo.toml" ]]; then
  cd "$ROOT_DIR/rust-src"
  if cargo check --quiet 2>/dev/null; then
    echo -e "  ${GREEN}cargo check ✅ 通过${NC}"
  else
    echo -e "  ${RED}cargo check ❌ 失败${NC}"
  fi
  
  test_count=$(cargo test --no-run 2>&1 | grep -oP '\d+ test' | head -1 || echo "?")
  echo -e "  测试: $test_count"
  cd "$ROOT_DIR"
else
  echo -e "  ${YELLOW}rust-src/Cargo.toml 不存在（项目未初始化）${NC}"
fi
echo ""

# ── 完成报告 ──────────────────────────────────────────────────────────────────
echo -e "${BOLD}✅ 已完成模块${NC}"
reports_dir="$ROOT_DIR/.rust-rewrite/reports"
if [[ -d "$reports_dir" ]] && [[ -n "$(ls -A "$reports_dir" 2>/dev/null)" ]]; then
  ls "$reports_dir"/*.md 2>/dev/null | xargs -I{} basename {} .md | sed 's/_done//' | while read -r r; do
    echo "  ✅ $r"
  done
else
  echo -e "  ${YELLOW}暂无已完成模块${NC}"
fi
echo ""
