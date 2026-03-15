#!/usr/bin/env bash
# =============================================================================
# Hook: TeammateIdle
# 退出码 2 = 阻止空闲，stdout 发回给 Teammate。
# 退出码 0 = 允许空闲。
# =============================================================================

set -euo pipefail

TEAMMATE="${TEAMMATE_NAME:-}"
TEAM_DIR="$(pwd)/.rust-rewrite/team"

[[ -z "$TEAMMATE" ]] && exit 0

MODULE=$(echo "$TEAMMATE" | sed 's/-engineer$//' | sed 's/-eng$//')
TASK_FILE="$TEAM_DIR/$MODULE/task.md"

[[ ! -f "$TASK_FILE" ]] && exit 0

PENDING=$(grep -c '^\- \[ \]' "$TASK_FILE" 2>/dev/null || echo 0)
BLOCKED=$(grep -c '\[BLOCKER\]' "$TASK_FILE" 2>/dev/null || echo 0)

if [[ "$PENDING" -gt 0 && "$BLOCKED" -eq 0 ]]; then
  echo "⚠️  你的 task.md 中还有 $PENDING 个未完成任务。"
  echo "请继续执行 $TASK_FILE 中状态为 '- [ ]' 的任务。"
  echo "完成后更新状态为 '- [x]'，并通过 mailbox 通知 rewrite-lead。"
  exit 2
fi

[[ "$BLOCKED" -gt 0 ]] && echo "有 $BLOCKED 个 [BLOCKER]，等待 rewrite-lead 处理。进入空闲。"

exit 0
