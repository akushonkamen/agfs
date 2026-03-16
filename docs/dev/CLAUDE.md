# AGFS Rust 重写团队 · 共享规范

> 所有 Agent（Leader + 所有 Teammate）自动加载本文件，严格遵守。

---

## 🎯 项目目标

将 [c4pt0r/agfs](https://github.com/c4pt0r/agfs) 完整重写为 Rust，保持：
- 所有 HTTP API 端点行为 1:1 兼容（`/api/v1/*`）
- 所有 plugin 功能 1:1 复刻
- 所有现有测试通过
- FUSE 挂载功能在 Linux 下正常工作
- agfs-shell（Python）和 agfs-mcp（Python）**不重写**，只需确保它们对接 Rust server 后行为不变

---

## 📁 目录结构

```
agfs/                              ← 项目根目录
  rust-src/                        ← 完整项目目录（Rust + Python 组件）
    Cargo.toml                     ← workspace
    agfs-server/                   ← Rust 服务端（对应 Go agfs-server）
    agfs-sdk/                      ← Rust SDK crate（对应 Go agfs-sdk）
    agfs-fuse/                     ← Rust FUSE（对应 Go agfs-fuse）
    agfs-mcp/                      ← Python MCP 集成（不重写）
    agfs-shell/                    ← Python shell（不重写）
    python-sdk/                    ← Python SDK（对应原 agfs-sdk/python）

.rust-rewrite/                     ← 团队协作工作区
  PHASES.md                        ← Leader 维护的 Phase 计划
  TEAM_ROSTER.md                   ← 成员清单
  team/{module}/
    discussion.md                  ← 双向讨论频道
    task.md                        ← 任务清单
  reports/                         ← 模块完成报告
```

---

## 🏗️ 架构速览（重写前必读）

### 核心抽象

**`FileSystem` trait**（对应 Go `filesystem.FileSystem` interface）：
所有 plugin 实现此 trait，是整个系统的核心抽象。

```
FileSystem
├── Create / Mkdir / Remove / RemoveAll
├── Read(path, offset, size) / Write(path, data, offset, flags)
├── ReadDir / Stat / Rename / Chmod
├── Open(→ Reader) / OpenWrite(→ Writer)
└── 可选扩展 trait：
    ├── Streamer    → OpenStream（streamfs 用）
    ├── Toucher     → Touch（heartbeatfs 用）
    ├── Symlinker   → Symlink / Readlink
    └── Truncater   → Truncate
```

**`ServicePlugin` trait**（对应 Go `plugin.ServicePlugin`）：
```
ServicePlugin
├── name() / validate(config) / initialize(config)
├── get_filesystem() → &dyn FileSystem
├── get_readme() / get_config_params()
└── shutdown()
```

**`MountableFS`**（对应 Go `mountablefs.MountableFS`）：
- 使用基数树（radix tree）做路径路由，将请求分发到对应 plugin
- 持有全局 handle ID（跨 plugin 唯一）
- 管理 symlink 映射表

**HTTP API**（`/api/v1/`）：
| 端点 | 方法 | 功能 |
|------|------|------|
| `/files` | GET/POST/PUT/DELETE | 文件 CRUD + 流式读 |
| `/directories` | GET/POST/DELETE | 目录操作 |
| `/stat` | GET | 文件元信息 |
| `/rename` | POST | 重命名 |
| `/chmod` | POST | 权限修改 |
| `/touch` | POST | 更新时间戳 |
| `/truncate` | POST | 截断文件 |
| `/symlink` | POST | 创建符号链接 |
| `/readlink` | GET | 读取符号链接 |
| `/grep` | POST | 文件内容搜索（支持流式 NDJSON） |
| `/digest` | POST | 文件哈希（xxh3 / md5） |
| `/handles/*` | * | 有状态文件句柄操作 |
| `/capabilities` | GET | 服务能力声明 |
| `/health` | GET | 健康检查 |
| `/plugins/*` | * | 插件管理 API |

### Plugins 清单（全部需要实现）

| Plugin | 说明 |
|--------|------|
| `memfs` | 内存文件系统 |
| `kvfs` | 基于 TiKV/内存的 KV 存储 |
| `queuefs` | 消息队列（SQLite/DB backend） |
| `heartbeatfs` | Agent 心跳/存活检测 |
| `streamfs` | 流式数据，多 reader fanout |
| `streamrotatefs` | 轮转流 |
| `localfs` | 挂载本地目录 |
| `s3fs` | S3 对象存储 |
| `sqlfs` | SQL 数据库文件系统 |
| `sqlfs2` | 改进版 SQL FS（MySQL/SQLite/TiDB） |
| `httpfs` | HTTP 请求映射为文件操作 |
| `proxyfs` | 代理到另一个 AGFS server |
| `vectorfs` | 向量搜索（embedding + TiDB） |
| `devfs` | /dev 设备文件（null/zero/random/urandom） |
| `serverinfofs` | 服务信息与流量统计 |
| `hellofs` | 示例 plugin |
| `gptfs` | GPT 集成 |
| WASM plugin | 外部 WASM plugin 加载（extism） |

---

## 🔧 Rust 技术栈（固定，不得擅自变更）

| 用途 | 选型 |
|------|------|
| HTTP 框架 | `axum` |
| 异步运行时 | `tokio` |
| 序列化 | `serde` / `serde_json` |
| 错误处理 | `thiserror` (库) / `anyhow` (应用) |
| 日志 | `tracing` / `tracing-subscriber` |
| 配置 | `serde_yaml` + 自定义解析 |
| 哈希 | `xxhash-rust` (xxh3) / `md5` |
| 正则 | `regex` |
| 基数树 | `radix_trie` 或 `matchit` |
| FUSE | `fuse3` (Linux) |
| S3 | `aws-sdk-s3` |
| SQL | `sqlx`（SQLite + MySQL） |
| WASM | `extism` |
| 向量/AI | `reqwest` (调外部 embedding API) |
| 文件句柄 | `dashmap` (并发 HashMap) |
| 测试 | `tokio-test` / `axum-test` |

---

## 📜 协作规范

### 文件所有权
- 每个 Teammate 只写自己负责的 `rust-src/{crate}/` 目录
- 原始 Go 代码（项目根下各子目录）只读，禁止修改
- 每次 git commit 前确认自己的模块 `CLAUDE.md` 已反映最新变更

### CLAUDE.md 更新时机
发现以下情况时**立即更新**模块 `CLAUDE.md`：
- 新增/变更公开 trait / struct / fn 签名
- 发现 Go→Rust 转换陷阱（特别是 Go interface 隐式实现 vs Rust 显式 trait）
- plugin 的初始化顺序、异步约束有特殊要求
- 与其他 crate 的依赖关系变化

### 跨模块协调
- 先读对方模块的 `rust-src/{crate}/CLAUDE.md` 了解公开 API
- 在对方的 `discussion.md` 末尾追加消息
- 有接口破坏性变更时，mailbox 通知 Leader + 受影响 Teammate

### 测试要求
- 每个 plugin 至少有基本的 unit test（Read/Write/ReadDir/Stat）
- API 层必须有 integration test（用 `axum-test` 或真实 HTTP 请求）
- agfs-shell 和 agfs-mcp 的现有测试必须在对接 Rust server 后通过

### Commit 规范

**Teammate 在完成每个任务后必须自己 commit**，然后再把 task.md 中任务状态标为 ✅：

```bash
# 只 add 自己负责的文件
git add rust-src/agfs-server/src/plugins/memfs/
git add .rust-rewrite/team/plugin-basic/task.md

# commit message 描述本次实际做了什么
git commit -m "[agfs-server] feat: implement memfs plugin with Read/Write/ReadDir"
```

格式：`[crate] type: 具体描述`
- crate：`agfs-sdk` / `agfs-server` / `agfs-fuse`
- type：`feat` / `fix` / `test` / `docs` / `refactor`
- 描述：说清楚做了什么，不要写"update"、"fix bug"这种无意义的内容

**TaskCompleted hook 会检查是否有未提交变更**——有未 commit 的文件会阻止任务完成，强制先 commit。

---

## ✅ 验收标准

每个模块完成须满足：
1. `cargo clippy -- -D warnings` 零警告
2. `cargo test` 所有 unit test 通过
3. 对应 integration test 通过
4. API 行为与 Go 原始实现一致（相同请求，相同响应格式）
5. `cargo doc --no-deps` 成功，公开 API 有文档注释

最终验收：
- 用 agfs-shell 连接 Rust server，所有示例命令正常工作
- `docker build` 成功，容器行为与原 Go 版本一致

---

## 🚫 禁止事项
- 不得修改 rust-src/agfs-mcp/ 和 rust-src/agfs-shell/（Python 代码不在重写范围）
- 不得在没有 Leader 裁决的情况下变更技术栈选型
- 不得手动删除 Go 源码，必须通过 `.claude/hooks/delete_go_src.sh` 脚本
