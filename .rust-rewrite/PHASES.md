# CtxFS (AGFS) Rust 重写 · Phase 进度追踪

> 项目已重命名：AGFS → CtxFS
> 原始项目：https://github.com/c4pt0r/agfs

---

## 项目结构调整

**实际目录结构**（与原 LEADER_PROMPT.md 不同）：
- `src/` - Rust workspace 根目录（不是 `rust-src/`）
- `src/sdk/` - `ctxfs-sdk` crate
- `src/server/` - `ctxfs-server` crate
- `src/fuse/` - `ctxfs-fuse` crate
- `src/python-sdk/` - Python SDK（不重写）
- `src/mcp/` - MCP 集成（不重写）
- `src/shell/` - agfs-shell（不重写）

---

## Phase 进度

| Phase | 状态 | 负责人 | 完成时间 | 备注 |
|-------|------|--------|----------|------|
| Phase 0 | ✅ 完成 | infra-engineer | - | Workspace 初始化完成 |
| Phase 1 | 🔄 进行中 | core-engineer | - | 核心类型与 FileSystem trait 已定义 |
| Phase 2 | ⏳ 待开始 | server-engineer | - | MountableFS 与 HTTP 层 |
| Phase 3 | ⏳ 待开始 | plugin-basic-engineer | - | 基础 Plugins |
| Phase 4 | ⏳ 待开始 | plugin-stream-engineer | - | 流与队列 Plugins |
| Phase 5 | ⏳ 待开始 | plugin-storage-engineer | - | 存储 Plugins |
| Phase 6 | ⏳ 待开始 | plugin-advanced-engineer | - | 高级 Plugins |
| Phase 7 | ⏳ 待开始 | fuse-engineer | - | FUSE 客户端 |
| Phase 8 | ⏳ 待开始 | integration-engineer | - | 端到端验收 |
| Phase 9 | ⏳ 待开始 | - | - | 清理 |

---

## Phase 详情

### Phase 0：Workspace 初始化 ✅
- ✅ 创建 Cargo workspace（`src/Cargo.toml`）
- ✅ 配置所有依赖（axum, tokio, serde, thiserror 等）
- ✅ 创建 `rustfmt.toml`、`.clippy.toml`
- ✅ 创建 CI（`.github/workflows/`）
- ✅ 创建 `Dockerfile` 骨架
- ✅ 建立测试目录结构

**验收**：`cargo build` 通过 ✅

---

### Phase 1：核心类型与 FileSystem trait 🔄
**Teammate**：`core-engineer`

**任务清单**：
- ✅ `agfs-sdk` crate 基础结构
- ✅ `FileSystem` trait 定义
- ✅ `ServicePlugin` trait 定义
- ✅ `FileInfo`、`MetaData`、`WriteFlag`、`OpenFlag` 等类型
- ✅ `AgfsError` 统一错误类型
- ✅ SDK HTTP Client (`client.rs`)
- ⏳ 更多单元测试

**Go 参考**：
- `agfs-server/pkg/filesystem/filesystem.go`
- `agfs-server/pkg/filesystem/errors.go`
- `agfs-server/pkg/plugin/plugin.go`
- `agfs-sdk/go/types.go`、`agfs-sdk/go/client.go`

**验收标准**：
- [ ] 所有 trait 和类型可编译
- [ ] SDK client 有 unit tests
- [ ] `cargo test -p ctxfs-sdk` 通过

---

### Phase 2：MountableFS 与 HTTP 层 ⏳
**Teammates**：`server-engineer`

**任务**：
- ✅ `MountableFS` 基数树路由框架
- ✅ Traffic monitor
- ✅ 配置加载（YAML）
- ✅ HTTP 路由框架
- ⏳ 完整的 HTTP handlers 实现
- ⏳ Plugin handler API
- ⏳ 流式读实现
- ⏳ Grep 实现

**验收标准**：
- [ ] 空 plugin 可挂载
- [ ] 所有 API 端点返回正确格式
- [ ] Integration tests 覆盖所有端点

---

### Phase 3-9：待详细规划

（后续 Phase 根据 LEADER_PROMPT.md 规范推进）

---

## 更新日志

- **2025-03-16**: 项目重命名为 CtxFS，Phase 0-1 基础框架已就绪
