# CtxFS Rust 重写 · Team 名单

> 最后更新：2025-03-16

---

## 当前 Team

**Phase**: 1 → 2 过渡期
**Team Lead**: `rewrite-lead`
**活跃 Teammates**: 无

---

## Teammate 名单

| 代号 | 角色 | Phase | 状态 | 备注 |
|------|------|-------|------|------|
| `infra-engineer` | 基础设施 | Phase 0 | ✅ 已解散 | Workspace 初始化完成 |
| `core-engineer` | 核心类型 | Phase 1 | 🔄 待检查 | 需要验证完成状态 |
| `server-engineer` | 服务端框架 | Phase 2 | ⏳ 待召唤 | HTTP 层实现 |
| `plugin-basic-engineer` | 基础插件 | Phase 3 | ⏳ 待召唤 | memfs, kvfs 等 |
| `plugin-stream-engineer` | 流插件 | Phase 4 | ⏳ 待召唤 | streamfs, queuefs |
| `plugin-storage-engineer` | 存储插件 | Phase 5 | ⏳ 待召唤 | localfs, s3fs, sqlfs |
| `plugin-advanced-engineer` | 高级插件 | Phase 6 | ⏳ 待召唤 | httpfs, proxyfs, vectorfs |
| `fuse-engineer` | FUSE 客户端 | Phase 7 | ⏳ 待召唤 | FUSE 挂载 |
| `integration-engineer` | 集成测试 | Phase 8 | ⏳ 待召唤 | 端到端验收 |

---

## 模块所有权

| Crate | 负责人 | 文件路径 |
|-------|--------|----------|
| `ctxfs-sdk` | `core-engineer` | `src/sdk/` |
| `ctxfs-server` | `server-engineer` + plugin engineers | `src/server/` |
| `ctxfs-fuse` | `fuse-engineer` | `src/fuse/` |

---

## 变更记录

- **2025-03-16**: 初始名单，项目从 AGFS 重命名为 CtxFS
