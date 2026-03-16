# CtxFS (AGFS) Rust 重写 · Phase 进度追踪

> 项目已重命名：AGFS → CtxFS
> 原始项目：https://github.com/c4pt0r/agfs

---

## 项目结构调整

**实际目录结构**（与原 LEADER_PROMPT.md 不同）：
- `src/` - Rust workspace 根目录
- `src/sdk/` - `ctxfs-sdk` crate
- `src/server/` - `ctxfs-server` crate
- `src/fuse/` - `ctxfs-fuse` crate
- `src/python-sdk/` - Python SDK（不重写）
- `src/mcp/` - MCP 集成（不重写）
- `src/shell/` - agfs-shell（不重写）

---

## Phase 进度总览

| Phase | 状态 | 负责人 | 完成时间 | 备注 |
|-------|------|--------|----------|------|
| Phase 0 | ✅ 完成 | infra-engineer | 2025-03-17 | Workspace 初始化完成 |
| Phase 1 | ✅ 完成 | - | - | 核心类型已实现 |
| Phase 2 | ✅ 完成 | - | - | MountableFS + HTTP 层已实现 |
| Phase 3 | ✅ 完成 | - | - | 基础 Plugins 已实现 |
| Phase 4 | ✅ 完成 | - | - | 流 Plugins 已实现 |
| Phase 5 | ✅ 完成 | - | - | 存储 Plugins 已实现 |
| Phase 6 | ✅ 完成 | - | - | 高级 Plugins 已实现 |
| Phase 7 | ✅ 完成 | - | 2025-03-17 | FUSE 客户端已实现并编译通过 |
| Phase 8 | ⏳ 待开始 | integration-engineer | - | 端到端验收 |
| Phase 9 | ⏳ 待开始 | - | - | 清理 Go 源码 |

---

## Phase 详情

### Phase 0：Workspace 初始化 ✅
- ✅ 创建 Cargo workspace（`src/Cargo.toml`）
- ✅ 配置所有依赖（axum, tokio, serde, thiserror 等）
- ✅ 创建 `rustfmt.toml`、`.clippy.toml`
- ✅ 创建 CI（`.github/workflows/`）
- ✅ 创建 `Dockerfile`
- ✅ Git 仓库初始化（.gitignore, .rust-rewrite/）

**验收**：`cargo build` 通过 ✅

---

### Phase 1：核心类型与 FileSystem trait ✅
**文件**: `src/sdk/src/`

- ✅ `FileSystem` trait (`filesystem.rs`, 248 lines)
- ✅ `ServicePlugin` trait (`plugin.rs`, 125 lines)
- ✅ `FileInfo`、`MetaData`、`WriteFlag`、`OpenFlag` 等类型 (`types.rs`, 384 lines)
- ✅ `AgfsError` 统一错误类型 (`error.rs`, 98 lines)
- ✅ SDK HTTP Client (`client.rs`, 620 lines)
- ✅ 模块导出 (`lib.rs`)

**验收**：编译通过 ✅

---

### Phase 2：MountableFS 与 HTTP 层 ✅
**文件**: `src/server/src/`

- ✅ `MountableFS` 基数树路由 (`mountablefs.rs`, 990 lines)
- ✅ HTTP 路由框架 (`router.rs`, 55 lines)
- ✅ Traffic monitor (`traffic_monitor.rs`, 123 lines)
- ✅ 配置加载 (`config.rs`, 30 lines)
- ✅ HTTP handlers:
  - ✅ `files.rs` (314 lines) - 文件 CRUD
  - ✅ `directories.rs` (96 lines) - 目录操作
  - ✅ `handles.rs` (192 lines) - 有状态句柄
  - ✅ `plugins.rs` (156 lines) - 插件管理
  - ✅ `grep.rs` (119 lines) - 搜索
  - ✅ `operations.rs` (267 lines) - 其他操作
  - ✅ `system.rs` (64 lines) - 系统端点
  - ✅ `response.rs` (383 lines) - 响应处理

**验收**：所有 API 端点实现 ✅

---

### Phase 3：基础 Plugins ✅
**文件**: `src/server/src/plugins/`

- ✅ `devfs.rs` (7,474 bytes) - 设备文件
- ✅ `memfs.rs` (14,689 bytes) - 内存文件系统
- ✅ `hellofs.rs` (4,063 bytes) - 示例插件
- ✅ `kvfs.rs` (7,560 bytes) - KV 存储
- ✅ `empty.rs` (5,026 bytes) - 空实现

**验收**：所有基础 plugin 实现 ✅

---

### Phase 4：流与队列 Plugins ✅
**文件**: `src/server/src/plugins/`

- ✅ `streamfs.rs` (7,404 bytes) - 流式文件系统
- ✅ `streamrotatefs.rs` (9,277 bytes) - 轮转流
- ✅ `queuefs.rs` (10,092 bytes) - 消息队列

**验收**：流插件实现 ✅

---

### Phase 5：存储 Plugins ✅
**文件**: `src/server/src/plugins/`

- ✅ `localfs.rs` (28,362 bytes) - 本地目录挂载
- ✅ `s3fs.rs` (21,272 bytes) - S3 对象存储
- ✅ `sqlfs.rs` (41,056 bytes) - SQL 文件系统
- ✅ `sqlfs2.rs` (23,730 bytes) - 改进版 SQL FS

**验收**：存储插件实现 ✅

---

### Phase 6：高级 Plugins ✅
**文件**: `src/server/src/plugins/`

- ✅ `httpfs.rs` (10,964 bytes) - HTTP 代理
- ✅ `proxyfs.rs` (15,192 bytes) - AGFS 代理
- ✅ `gptfs.rs` (15,563 bytes) - GPT 集成
- ✅ `vectorfs.rs` (19,761 bytes) - 向量搜索

**验收**：高级插件实现 ✅

---

### Phase 7：FUSE 客户端 🔄
**文件**: `src/fuse/`

**任务清单**：
- [ ] 检查现有 FUSE 实现状态
- [ ] 实现 metadata cache
- [ ] 实现 directory cache
- [ ] 实现 handle manager
- [ ] 命令行参数解析
- [ ] Linux 测试

---

### Phase 8：端到端验收 ⏳
**任务清单**：
- [ ] 运行集成测试（需要启动服务器）
- [ ] agfs-shell 连接测试
- [ ] Docker 构建测试
- [ ] 与 Go 版本行为对比
- [ ] 性能基准测试

---

### Phase 9：清理 ⏳
**任务**：
```bash
bash .claude/hooks/delete_go_src.sh
```
删除 Go 源码（除 mcp/ 和 shell/），更新 README。

---

## 下一步行动

1. **立即**：检查 FUSE 模块实现状态
2. **本周**：完成 Phase 7 (FUSE)
3. **下周**：Phase 8 端到端验收

---

## 更新日志

- **2025-03-17**: 项目重命名为 CtxFS，Phase 0-6 基本完成，Phase 7-9 待推进
- **2025-03-16**: 创建 .rust-rewrite 工作区
