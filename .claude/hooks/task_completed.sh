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

# ── 门禁 4：检查 Teammate 是否已 commit ───────────────────────────────────────
# Teammate 负责自己 commit；有未提交变更则阻止任务完成，强制先 commit
if git -C "$(pwd)" rev-parse --git-dir > /dev/null 2>&1; then
  if ! git -C "$(pwd)" diff --quiet 2>/dev/null || \
     ! git -C "$(pwd)" diff --cached --quiet 2>/dev/null; then
    echo "❌ 有未提交的变更，请先 git commit 再标记任务完成。"
    echo ""
    echo "未提交文件："
    git -C "$(pwd)" status --short 2>/dev/null | head -20
    echo ""
    echo "Commit 格式：git commit -m '[crate] feat/fix: 描述本次做了什么'"
    exit 2
  fi
fi

exit 0
