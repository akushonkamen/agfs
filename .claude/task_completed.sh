#!/usr/bin/env bash
# =============================================================================
# Hook: TaskCompleted
# 退出码 2 = 阻止完成，stdout 发回给 Teammate。
# 退出码 0 = 允许完成。
# =============================================================================

set -euo pipefail

RUST_SRC="$(pwd)/rust-src"
TASK_TITLE="${TASK_TITLE:-}"
TEAMMATE="${TEAMMATE_NAME:-unknown}"
MODULE=$(echo "$TEAMMATE" | sed 's/-engineer$//' | sed 's/-eng$//')

if [[ -f "$RUST_SRC/Cargo.toml" ]]; then
  # ── 门禁 1：基础编译检查 ──────────────────────────────────────────────────────
  if echo "$TASK_TITLE" | grep -qiE "(impl|struct|trait|fn |mod |crate|rust|feature|service|handler|repo)"; then
    if ! cargo check --manifest-path "$RUST_SRC/Cargo.toml" --quiet 2>/tmp/cargo_check_err; then
      echo "❌ cargo check 失败，请修复后重新标记完成："
      cat /tmp/cargo_check_err
      exit 2
    fi
  fi

  # ── 门禁 2：测试检查 ──────────────────────────────────────────────────────────
  if echo "$TASK_TITLE" | grep -qiE "(test|spec|验收|完成)"; then
    if ! cargo test --manifest-path "$RUST_SRC/Cargo.toml" --quiet "$MODULE" 2>/tmp/cargo_test_err \
       && ! cargo test --manifest-path "$RUST_SRC/Cargo.toml" --quiet 2>/tmp/cargo_test_err; then
      echo "❌ cargo test 失败，请修复后重新标记完成："
      head -40 /tmp/cargo_test_err
      exit 2
    fi
  fi

  # ── 门禁 3：Clippy ────────────────────────────────────────────────────────────
  if echo "$TASK_TITLE" | grep -qiE "(clippy|lint|模块完成|phase complete)"; then
    if ! cargo clippy --manifest-path "$RUST_SRC/Cargo.toml" --quiet -- -D warnings 2>/tmp/clippy_err; then
      echo "❌ cargo clippy 有警告，请修复后重新标记完成："
      head -40 /tmp/clippy_err
      exit 2
    fi
  fi
fi

# ── 自动 git commit ────────────────────────────────────────────────────────────
if git -C "$(pwd)" rev-parse --git-dir > /dev/null 2>&1; then
  if [[ -n "$MODULE" ]]; then
    git -C "$(pwd)" add \
        "rust-src/src/$MODULE/" \
        "rust-src/tests/" \
        ".rust-rewrite/team/$MODULE/" \
        "rust-src/src/$MODULE/CLAUDE.md" 2>/dev/null || true
  else
    git -C "$(pwd)" add "rust-src/" ".rust-rewrite/" 2>/dev/null || true
  fi

  if ! git -C "$(pwd)" diff --cached --quiet 2>/dev/null; then
    SAFE_TITLE=$(echo "$TASK_TITLE" | head -c 60 | tr '\n' ' ')
    git -C "$(pwd)" commit \
        -m "[$MODULE] ${SAFE_TITLE}" \
        --author="$TEAMMATE <agent@rewrite-team>" \
        --no-verify 2>/dev/null || true
  fi
fi

# ── Phase 完成时打 tag ─────────────────────────────────────────────────────────
if echo "$TASK_TITLE" | grep -qiE "(phase.*(complete|完成)|all.*done|最终验收)"; then
  PHASE_TAG="phase-$(date '+%Y%m%d-%H%M')-complete"
  git -C "$(pwd)" tag "$PHASE_TAG" 2>/dev/null || true
fi

exit 0
