# Phase 1: 核心类型与 FileSystem trait - 讨论频道

> 双向讨论频道，记录技术决策和问题

## 设计决策

### FileSystem trait 设计
- 同步 trait（与 Go 版本一致）
- 返回 `Result<T, AgfsError>`
- `Send + Sync` 约束

### StreamReader 设计
- 使用 `tokio::sync::broadcast` 实现多 reader fanout（Phase 4 时实现）

## 待讨论问题
（暂无）
