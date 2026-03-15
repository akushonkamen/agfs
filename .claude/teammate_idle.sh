#!/usr/bin/env bash
# =============================================================================
# Hook: TeammateIdle
# 当一个 Teammate 即将进入空闲状态时触发。
# 退出码 2 = 阻止空闲，把 stdout 作为反馈发回给 Teammate，让它继续工作。
# 退出码 0 = 允许空闲（正常结束）。
# =============================================================================
#
# 环境变量（Claude Code 注入）：
#   TEAMMATE_NAME  - 即将空闲的 Teammate 名称
#   TEAM_NAME      - 团队名称
#
# 逻辑：
#   检查该 Teammate 对应的 task.md 是否还有未完成的任务。
#   如果有，阻止空闲并提醒它继续。
# =============================================================================

set -euo pipefail

TEAMMATE="${TEAMMATE_NAME:-}"
TEAM_DIR="$(pwd)/.rust-rewrite/team"

if [[ -z "$TEAMMATE" ]]; then
  exit 0
fi

# 从 Teammate 名推断模块名（e.g. "db-engineer" → "db"，"types-engineer" → "types"）
MODULE=$(echo "$TEAMMATE" | sed 's/-engineer$//' | sed 's/-eng$//')
TASK_FILE="$TEAM_DIR/$MODULE/task.md"

# 如果没有 task.md，允许空闲
if [[ ! -f "$TASK_FILE" ]]; then
  exit 0
fi

# 检查是否有未完成的任务（- [ ] 开头的行）
PENDING=$(grep -c '^\- \[ \]' "$TASK_FILE" 2>/dev/null || echo 0)
BLOCKED=$(grep -c '\[BLOCKER\]' "$TASK_FILE" 2>/dev/null || echo 0)

if [[ "$PENDING" -gt 0 && "$BLOCKED" -eq 0 ]]; then
  echo "⚠️  你的 task.md 中还有 $PENDING 个未完成任务。"
  echo ""
  echo "请继续执行 $TASK_FILE 中状态为 '- [ ]' 的任务。"
  echo "完成后更新任务状态为 '- [x]'，并通过 mailbox 通知 rewrite-lead。"
  echo ""
  echo "如果实际上已完成但 task.md 未更新，请先同步状态再空闲。"
  exit 2
fi

if [[ "$BLOCKED" -gt 0 ]]; then
  # 有阻塞，允许空闲但给出提示（阻塞需要 Leader 介入，不应强制继续）
  echo "你有 $BLOCKED 个 [BLOCKER] 未解决，等待 rewrite-lead 处理。进入空闲。"
  exit 0
fi

# 所有任务完成，允许空闲
exit 0
