#!/usr/bin/env bash
# =============================================================================
# 创建新 Teammate 工作区（Leader 辅助脚本）
# 用法: ./scripts/spawn_teammate.sh <module-name> <teammate-name> <phase-number>
# 示例: ./scripts/spawn_teammate.sh db-layer db-engineer 2
# =============================================================================

set -euo pipefail

MODULE="${1:-}"
TEAMMATE="${2:-}"
PHASE="${3:-1}"

[[ -z "$MODULE" ]] && { echo "Usage: $0 <module-name> <teammate-name> [phase]"; exit 1; }
[[ -z "$TEAMMATE" ]] && TEAMMATE="${MODULE}-engineer"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TEAM_DIR="$ROOT_DIR/.rust-rewrite/team"
TEMPLATE_DIR="$TEAM_DIR/template"
MODULE_DIR="$TEAM_DIR/$MODULE"
DATE=$(date '+%Y-%m-%d')

GREEN='\033[0;32m'; BLUE='\033[0;34m'; YELLOW='\033[1;33m'; NC='\033[0m'

# ── 创建模块工作区 ─────────────────────────────────────────────────────────────
echo -e "${BLUE}[spawn]${NC} 为 $TEAMMATE 创建模块工作区: $MODULE"

# ── 检查模块 CLAUDE.md ────────────────────────────────────────────────────────
MODULE_CLAUDE="$ROOT_DIR/rust-src/src/$MODULE/CLAUDE.md"
GO_MODULE_CLAUDE="$ROOT_DIR/go-src/$MODULE/CLAUDE.md"

if [[ -f "$MODULE_CLAUDE" ]]; then
  echo -e "${GREEN}[ok]${NC}   发现模块 CLAUDE.md：rust-src/src/$MODULE/CLAUDE.md"
else
  echo -e "${YELLOW}[warn]${NC} 模块 CLAUDE.md 不存在，将创建空模板"
  echo -e "       Leader 应在 spawn Teammate 前填写：rust-src/src/$MODULE/CLAUDE.md"
  mkdir -p "$ROOT_DIR/rust-src/src/$MODULE"
  cat > "$MODULE_CLAUDE" <<TMPL
# $MODULE 模块规范

> **维护者**：$TEAMMATE
> **创建时间**：$DATE
> **所属 Phase**：Phase $PHASE

## 模块职责

{待填写：从 Go 源码分析得出}

## 对外公共 API

{待 Teammate 实现后完善}

\`\`\`rust
// pub trait / pub struct / pub fn 清单
\`\`\`

## 模块内部约定

- **错误处理**：{待填写}
- **异步策略**：{待填写}
- **命名规范**：遵循根 CLAUDE.md，补充模块特定规则如有

## 与其他模块的依赖

- 依赖：{module}（原因：）
- 被依赖：{module}（原因：）

## Go→Rust 转换注意事项

{待 Leader 分析 go-src/$MODULE/ 后填写}
TMPL
  echo -e "${YELLOW}[todo]${NC} 请 Leader 填写：rust-src/src/$MODULE/CLAUDE.md"
fi

if [[ -f "$GO_MODULE_CLAUDE" ]]; then
  echo -e "${GREEN}[ok]${NC}   发现 Go 侧模块 CLAUDE.md：go-src/$MODULE/CLAUDE.md（Teammate 可参考）"
fi
echo ""

# discussion.md
sed \
  -e "s/{module-name}/$MODULE/g" \
  -e "s/{teammate-name}/$TEAMMATE/g" \
  -e "s/{date}/$DATE/g" \
  "$TEMPLATE_DIR/discussion.md" > "$MODULE_DIR/discussion.md"

# task.md
sed \
  -e "s/{module-name}/$MODULE/g" \
  -e "s/{teammate-name}/$TEAMMATE/g" \
  -e "s/{date}/$DATE/g" \
  -e "s/{n}/$PHASE/g" \
  "$TEMPLATE_DIR/task.md" > "$MODULE_DIR/task.md"

# 创建 rust-src 对应目录
mkdir -p "$ROOT_DIR/rust-src/src/$MODULE"

echo -e "${GREEN}[ok]${NC}   工作区创建完成"
echo ""
echo "📁 文件位置："
echo "   .rust-rewrite/team/$MODULE/discussion.md"
echo "   .rust-rewrite/team/$MODULE/task.md"
echo "   rust-src/src/$MODULE/ (源码目录)"
echo ""
echo -e "${YELLOW}📋 Leader 下一步：${NC}"
echo "   1. 在 .rust-rewrite/team/$MODULE/task.md 中填写具体任务"
echo "   2. 在 .rust-rewrite/team/$MODULE/discussion.md 中填写项目背景"
echo "   3. 更新 .rust-rewrite/TEAM_ROSTER.md"
echo "   4. 使用以下 Prompt spawn $TEAMMATE："
echo ""
echo "   ┌─────────────────────────────────────────────────────────────"
echo "   │ 你是 $MODULE 模块的负责工程师，代号 $TEAMMATE。"
echo "   │ 启动时第一件事，按顺序阅读："
echo "   │   1. rust-src/src/$MODULE/CLAUDE.md   ← 你的模块规范"
echo "   │   2. .rust-rewrite/team/$MODULE/task.md"
echo "   │   3. .rust-rewrite/team/$MODULE/discussion.md"
echo "   │   4. go-src/$MODULE/（参考 Go 原始实现）"
echo "   │ 跨模块依赖时，先读对方的 rust-src/src/{other}/CLAUDE.md。"
echo "   │ 任务完成后通过 mailbox 通知 rewrite-lead。"
echo "   └─────────────────────────────────────────────────────────────"

# ── 更新 TEAM_ROSTER.md ────────────────────────────────────────────────────────
ROSTER="$ROOT_DIR/.rust-rewrite/TEAM_ROSTER.md"
if [[ -f "$ROSTER" ]]; then
  # 在"当前存活成员"表格中插入新行（在空行前）
  sed -i "s/| \*(空)\* | - | - | - | - |/| $TEAMMATE | $MODULE | Phase $PHASE | 🟡 进行中 | $DATE |/" "$ROSTER" 2>/dev/null || true
  echo ""
  echo -e "${GREEN}[ok]${NC}   TEAM_ROSTER.md 已更新"
fi
