# AGFS Rust 重写 Phase 计划

> 状态: `pending` | `in_progress` | `completed` | `blocked`

| Phase | 名称 | 状态 | Teammate | 开始时间 | 完成时间 |
|-------|------|------|----------|----------|----------|
| 0 | Workspace 初始化 | completed | infra-engineer | 2025-03-15 | 2025-03-15 |
| 1 | 核心类型与 FileSystem trait | completed | core-engineer | 2025-03-15 | 2025-03-15 |
| 2 | MountableFS 与 HTTP 层 | completed | server-engineer | 2025-03-15 | 2025-03-15 |
| 3 | 基础 Plugins（无外部依赖） | completed | plugin-basic-engineer | 2025-03-15 | 2025-03-15 |
| 4 | 流与队列 Plugins | completed | plugin-stream-engineer | 2025-03-15 | 2025-03-15 |
| 5 | 存储 Plugins（外部依赖） | completed | plugin-storage-engineer | 2025-03-15 | 2025-03-15 |
| 6 | 高级 Plugins | completed | plugin-advanced-engineer | 2025-03-15 | 2025-03-15 |
| 7 | FUSE 客户端 | completed | fuse-engineer | 2025-03-15 | 2025-03-15 |
| 8 | 端到端验收 | completed | integration-engineer | 2025-03-16 | 2025-03-16 |
| 9 | 清理（删除 Go 源码） | completed | - | 2025-03-16 | 2025-03-16 |
| 9 | 清理（删除 Go 源码） | pending | - | - | - |

---

## Phase 0：Workspace 初始化

**Teammate**: `infra-engineer`

**任务**:
- 创建 Cargo workspace（`rust-src/Cargo.toml`），成员：`agfs-server`、`agfs-sdk`、`agfs-fuse`
- 配置所有依赖（见 CLAUDE.md 技术栈表）
- 设置 `rustfmt.toml`、`.clippy.toml`
- 创建 CI（`.github/workflows/rust.yml`）
- 创建 `Dockerfile` 骨架（参考原 Go Dockerfile）
- 建立测试目录结构

**验收**: `cargo build` 通过（允许空实现）

---

## Phase 1：核心类型与 FileSystem trait

**Teammate**: `core-engineer`

**验收**: 所有 trait 和类型可编译，SDK client 有 unit tests

---

## Phase 2：MountableFS 与 HTTP 层

**Teammate**: `server-engineer`

**验收**: 空 plugin 可挂载，所有 API 端点返回正确格式

---

## Phase 3：基础 Plugins（无外部依赖）

**Teammate**: `plugin-basic-engineer`

**任务**: devfs, memfs, hellofs, heartbeatfs, serverinfofs, kvfs

**验收**: 所有 plugin unit tests 通过

---

## Phase 4：流与队列 Plugins

**Teammate**: `plugin-stream-engineer`

**任务**: streamfs, streamrotatefs, queuefs

---

## Phase 5：存储 Plugins（外部依赖）

**Teammate**: `plugin-storage-engineer`

**任务**: localfs, s3fs, sqlfs, sqlfs2

---

## Phase 6：高级 Plugins

**Teammate**: `plugin-advanced-engineer`

**任务**: httpfs, proxyfs, gptfs, vectorfs, WASM loader

---

## Phase 7：FUSE 客户端

**Teammate**: `fuse-engineer`

---

## Phase 8：端到端验收

**Teammate**: `integration-engineer`

---

## Phase 9：清理

**任务**（Leader 直接执行）: 删除 Go 源码
