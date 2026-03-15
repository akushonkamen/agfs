# agfs-server · 模块规范

> **维护者**：server-engineer（Phase 2） + plugin-*-engineer（Phase 3-6）
> **对应 Go 代码**：`agfs-server/`

---

## 模块职责

`agfs-server` 是核心 crate，包含：
1. **MountableFS**：基数树路由，将路径分发到对应 plugin 的 FileSystem 实现
2. **HTTP 服务**：axum 路由，实现所有 `/api/v1/*` 端点
3. **所有 Plugins**：memfs、kvfs、queuefs 等（每个 plugin 在 `src/plugins/{name}/` 下）
4. **配置加载**：YAML 配置文件解析
5. **Plugin 管理 API**：动态挂载/卸载

---

## Crate 内部结构

```
agfs-server/src/
├── main.rs                  ← 入口：解析 CLI，加载 config，启动 axum
├── lib.rs                   ← 公开模块
├── mountable_fs.rs          ← MountableFS 实现
├── config.rs                ← YAML 配置结构
├── error.rs                 ← HTTP 错误映射
├── handlers/
│   ├── mod.rs
│   ├── files.rs             ← /api/v1/files
│   ├── directories.rs       ← /api/v1/directories
│   ├── handles.rs           ← /api/v1/handles/*（有状态 handle）
│   ├── plugins.rs           ← /api/v1/plugins/*
│   └── ...                  ← stat, rename, grep, digest 等
├── traffic_monitor.rs       ← 流量统计（原子计数器）
└── plugins/
    ├── mod.rs               ← 注册所有 plugin factory
    ├── memfs/
    ├── kvfs/
    ├── queuefs/
    ├── heartbeatfs/
    ├── streamfs/
    ├── streamrotatefs/
    ├── localfs/
    ├── s3fs/
    ├── sqlfs/
    ├── sqlfs2/
    ├── httpfs/
    ├── proxyfs/
    ├── vectorfs/
    ├── devfs/
    ├── serverinfofs/
    ├── hellofs/
    ├── gptfs/
    └── wasm/                ← WASM plugin loader
```

---

## 对外公共 API

### MountableFS

```rust
pub struct MountableFS { /* ... */ }

impl MountableFS {
    pub fn new(pool_config: WasmPoolConfig) -> Self;
    pub fn register_plugin_factory(&mut self, name: &str, factory: PluginFactory);
    pub fn mount(&self, path: &str, plugin: Box<dyn ServicePlugin>) -> Result<(), AgfsError>;
    pub fn unmount(&self, path: &str) -> Result<(), AgfsError>;
    pub fn create_plugin(&self, name: &str) -> Option<Box<dyn ServicePlugin>>;
    pub fn list_mounts(&self) -> Vec<MountInfo>;
}

impl FileSystem for MountableFS { /* 路由到子 plugin */ }
impl Symlinker for MountableFS { /* 内置 symlink 表管理 */ }
```

### HTTP 路由（axum）

```rust
pub fn create_router(mfs: Arc<MountableFS>, monitor: Arc<TrafficMonitor>) -> Router;
```

---

## 模块约定

- **异步策略**：axum handler 是 async，但调用 `mfs.read()` 等 FileSystem 方法时用 `tokio::task::spawn_blocking` 包裹（因为 FileSystem trait 是同步的）
- **MountableFS 并发**：使用 `Arc<RwLock<RadixTree>>` 或 `Arc<DashMap>` 管理 mount 点；使用 `Arc<DashMap<i64, HandleInfo>>` 管理全局 handle
- **Plugin factory**：`type PluginFactory = fn() -> Box<dyn ServicePlugin>`
- **配置注入**：`mount_path` 自动注入到 plugin config 中（与 Go 版本一致）
- **流式读**：axum 支持 `axum::body::Body::from_stream`，用 `tokio::sync::mpsc` 实现 chunked transfer

---

## 与其他模块的依赖

- 依赖：`agfs-sdk`（所有 trait 和类型）
- 被依赖：无（是最顶层可执行 crate）

---

## Go→Rust 转换注意事项

1. **基数树选型**：Go 用 `hashicorp/go-immutable-radix`；Rust 用 `matchit` crate（axum 内置的路由器）或 `radix_trie`。推荐 `radix_trie`，它支持前缀匹配（MountableFS 需要找最长前缀匹配的 mount 点）
2. **atomic.Value（Go）→ ArcSwap（Rust）**：Go 的 `atomic.Value` 存 radix tree，Rust 用 `arc-swap` crate
3. **Global handle ID**：Go 用 `atomic.Int64`；Rust 用 `std::sync::atomic::AtomicI64`
4. **Plugin 特殊处理**：httpfs 需要注入 rootFS 引用（`Arc<MountableFS>`）；serverinfofs 需要注入 TrafficMonitor。Rust 用 trait object 或泛型参数实现
5. **Custom grep**：vectorfs 实现 `CustomGrepper` trait，MountableFS 在 grep 时检查是否实现了该 trait（用 `Any` downcast）
6. **Streaming**：Go 用 `http.Flusher`；axum 用 `axum::body::Body::from_stream` + tokio channel
