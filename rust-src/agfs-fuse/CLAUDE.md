# agfs-fuse · 模块规范

> **维护者**：fuse-engineer（Phase 7）
> **对应 Go 代码**：`agfs-fuse/`
> **平台限制**：仅 Linux（FUSE 依赖）

---

## 模块职责

将 AGFS HTTP API 挂载为本机 FUSE 文件系统，使任何程序可以用标准文件操作访问 AGFS。

```
应用程序  →  FUSE 内核模块  →  agfs-fuse  →  HTTP API  →  agfs-server
```

---

## Crate 内部结构

```
agfs-fuse/src/
├── main.rs           ← CLI 解析（--agfs-server-url, --mount, --cache-ttl, --debug）
├── fs.rs             ← AGFSFS：实现 fuse3 trait
├── node.rs           ← AgfsNode：文件/目录节点
├── handles.rs        ← HandleManager：管理打开的文件句柄
└── cache/
    ├── metadata.rs   ← MetadataCache（TTL-based）
    └── directory.rs  ← DirectoryCache（TTL-based）
```

---

## 对外公共 API（内部使用为主）

```rust
pub struct Config {
    pub server_url: String,
    pub cache_ttl: Duration,
    pub debug: bool,
}

pub struct AGFSFS {
    client: Arc<agfs_sdk::Client>,
    handles: HandleManager,
    meta_cache: MetadataCache,
    dir_cache: DirectoryCache,
    cache_ttl: Duration,
}
```

---

## 模块约定

- **FUSE crate**：使用 `fuse3`（支持 FUSE3 协议，比老版 `fuse` crate 更好）
- **异步**：`fuse3` 支持 async，与 tokio 集成
- **Cache TTL**：metadata cache 和 directory cache 默认 TTL 30s，`--cache-ttl` 参数可配置
- **Handle 管理**：open 时向 agfs-server 发 handle open 请求，close 时发 handle close；handle ID 由 server 分配
- **错误映射**：`AgfsError::NotFound` → `ENOENT`，`PermissionDenied` → `EACCES`，以此类推

---

## 与其他模块的依赖

- 依赖：`agfs-sdk`（Client 和所有类型）
- 依赖：`fuse3`（FUSE 内核接口）

---

## Go→Rust 转换注意事项

1. **Go fuse 库 → Rust fuse3**：Go 用 `hanwen/go-fuse/v2`，Rust 用 `fuse3` crate，API 结构相似但方法签名不同
2. **Go AGFSFS 继承 fs.Inode → Rust fuse3 的 Filesystem trait**：Rust 实现 `fuse3::raw::Filesystem` 或 `fuse3::path::Filesystem` trait
3. **Cache 实现**：Go 用 sync.Map + TTL；Rust 用 `DashMap<String, (T, Instant)>` 实现简单 TTL cache，或用 `moka` crate
4. **CI 注意**：FUSE 测试需要 Linux 环境且有 `/dev/fuse`；GitHub Actions 需要特殊配置（`--device /dev/fuse`）
