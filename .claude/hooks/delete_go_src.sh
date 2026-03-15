#!/usr/bin/env bash
# =============================================================================
# 删除 go-src（仅在全部 Phase 验收通过后执行）
# 用法: bash .claude/hooks/delete_go_src.sh
# =============================================================================

set -euo pipefail

RUST_SRC="$(pwd)/rust-src"
GO_SRC="$(pwd)/go-src"

echo "═══════════════════════════════════════════"
echo "  ⚠️  准备删除 go-src（不可逆操作）"
echo "═══════════════════════════════════════════"

# ── 安全检查：必须所有 Phase 都完成 ──────────────────
PENDING=$(grep -c '⬜\|🟡' "$(pwd)/.rust-rewrite/PHASES.md" 2>/dev/null || echo 99)
if [[ "$PENDING" -gt 0 ]]; then
  echo "❌ 还有未完成的 Phase，禁止删除 go-src。"
  echo "   请在 PHASES.md 中确认所有 Phase 状态为 ✅ 后再执行。"
  exit 1
fi

# ── 安全检查：全量测试必须通过 ────────────────────────
echo "▶ 最终全量测试..."
cargo test --manifest-path "$RUST_SRC/Cargo.toml" --quiet || {
  echo "❌ cargo test 未全部通过，禁止删除 go-src。"
  exit 1
}

# ── 打保留 tag ─────────────────────────────────────────
TAG="pre-delete-go-src-$(date '+%Y%m%d-%H%M')"
git -C "$(pwd)" add -A && git -C "$(pwd)" commit -m "chore: final state before go-src removal" --no-verify 2>/dev/null || true
git -C "$(pwd)" tag "$TAG"
echo "✅ 安全 tag 已打：$TAG（如需找回 go-src 可 checkout 此 tag）"

# ── 执行删除 ────────────────────────────────────────────
echo "▶ 删除 go-src/..."
rm -rf "$GO_SRC"
git -C "$(pwd)" add -A
git -C "$(pwd)" commit -m "chore: remove go-src after successful Rust rewrite" --no-verify

echo ""
echo "✅ go-src 已删除。Rust 重写完成。"
