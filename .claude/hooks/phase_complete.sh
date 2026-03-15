#!/usr/bin/env bash
# =============================================================================
# Phase 完成时执行（由 Leader 在 Phase 收尾时手动触发，或通过自定义命令调用）
# 用法: bash .claude/hooks/phase_complete.sh <phase-number> [staging-url]
# =============================================================================

set -euo pipefail

PHASE="${1:-?}"
STAGING_URL="${2:-}"
RUST_SRC="$(pwd)/rust-src"
DATE=$(date '+%Y-%m-%d %H:%M')

echo "═══════════════════════════════════════════"
echo "  Phase $PHASE 完成检查"
echo "═══════════════════════════════════════════"

# ── 1. 全量编译 ────────────────────────────────
echo "▶ cargo build..."
cargo build --manifest-path "$RUST_SRC/Cargo.toml" --quiet || { echo "❌ build 失败"; exit 1; }
echo "  ✅ build 通过"

# ── 2. 全量测试 ────────────────────────────────
echo "▶ cargo test..."
cargo test --manifest-path "$RUST_SRC/Cargo.toml" --quiet 2>&1 | tail -5
echo "  ✅ tests 通过"

# ── 3. Clippy ─────────────────────────────────
echo "▶ cargo clippy..."
cargo clippy --manifest-path "$RUST_SRC/Cargo.toml" --quiet -- -D warnings || { echo "❌ clippy 未通过"; exit 1; }
echo "  ✅ clippy 通过"

# ── 4. Staging 验证提示 ────────────────────────
if [[ -n "$STAGING_URL" ]]; then
  echo ""
  echo "▶ Staging 环境验证..."
  echo "  请确认以下端点可访问并行为正确："
  echo "  $STAGING_URL/health"
  echo ""
  echo "  提示：运行 docker compose up -d 拉起完整依赖栈后验证。"
  echo "  验证通过后继续下一 Phase；发现问题在 .rust-rewrite/reports/ 记录。"
fi

# ── 5. 打 Phase tag ────────────────────────────
TAG="phase-${PHASE}-complete"
git -C "$(pwd)" tag "$TAG" 2>/dev/null && echo "  ✅ git tag: $TAG" || echo "  ⚠️  tag 已存在，跳过"

# ── 6. 更新 PHASES.md ─────────────────────────
sed -i "s/Phase $PHASE | 🟡 进行中/Phase $PHASE | ✅ 完成 ($DATE)/" \
    "$(pwd)/.rust-rewrite/PHASES.md" 2>/dev/null || true

echo ""
echo "✅ Phase $PHASE 验收完成，可以进入下一 Phase。"
