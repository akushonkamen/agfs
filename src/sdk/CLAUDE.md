# agfs-sdk · 模块规范

> **维护者**：core-engineer（Phase 1）
> **对应 Go 代码**：`agfs-sdk/go/`、`agfs-server/pkg/filesystem/`、`agfs-server/pkg/plugin/`

---

## 模块职责

`agfs-sdk` 是整个 Rust 重写的基础 crate，提供：
1. **核心 trait 定义**：`FileSystem`、`ServicePlugin` 及所有可选扩展 trait
2. **共享类型**：`FileInfo`、`MetaData`、`WriteFlag`、`OpenFlag`、`AgfsError` 等
3. **HTTP Client**：供 agfs-fuse 和外部使用者调用 agfs-server

所有其他 crate 都依赖 `agfs-sdk`。

---

## 对外公共 API

### 核心 trait

```rust
pub trait FileSystem: Send + Sync {
    fn create(&self, path: &str) -> Result<(), AgfsError>;
    fn mkdir(&self, path: &str, perm: u32) -> Result<(), AgfsError>;
    fn remove(&self, path: &str) -> Result<(), AgfsError>;
    fn remove_all(&self, path: &str) -> Result<(), AgfsError>;
    fn read(&self, path: &str, offset: i64, size: i64) -> Result<Vec<u8>, AgfsError>;
    fn write(&self, path: &str, data: &[u8], offset: i64, flags: WriteFlag) -> Result<i64, AgfsError>;
    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>, AgfsError>;
    fn stat(&self, path: &str) -> Result<FileInfo, AgfsError>;
    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), AgfsError>;
    fn chmod(&self, path: &str, mode: u32) -> Result<(), AgfsError>;
    fn open(&self, path: &str) -> Result<Box<dyn Read + Send>, AgfsError>;
    fn open_write(&self, path: &str) -> Result<Box<dyn Write + Send>, AgfsError>;
}

// 可选扩展 trait（plugin 按需实现）
pub trait Streamer: FileSystem {
    fn open_stream(&self, path: &str) -> Result<Box<dyn StreamReader>, AgfsError>;
}
pub trait Toucher: FileSystem {
    fn touch(&self, path: &str) -> Result<(), AgfsError>;
}
pub trait Symlinker: FileSystem {
    fn symlink(&self, target: &str, link: &str) -> Result<(), AgfsError>;
    fn readlink(&self, link: &str) -> Result<String, AgfsError>;
}
pub trait Truncater: FileSystem {
    fn truncate(&self, path: &str, size: i64) -> Result<(), AgfsError>;
}

pub trait ServicePlugin: Send + Sync {
    fn name(&self) -> &str;
    fn validate(&self, config: &HashMap<String, Value>) -> Result<(), AgfsError>;
    fn initialize(&mut self, config: HashMap<String, Value>) -> Result<(), AgfsError>;
    fn get_filesystem(&self) -> &dyn FileSystem;
    fn get_readme(&self) -> &str;
    fn get_config_params(&self) -> Vec<ConfigParameter>;
    fn shutdown(&mut self) -> Result<(), AgfsError>;
}
```

### 类型

```rust
pub struct FileInfo {
    pub name: String,
    pub size: i64,
    pub mode: u32,
    pub mod_time: DateTime<Utc>,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub meta: MetaData,
}

pub struct MetaData {
    pub name: String,
    pub r#type: String,
    pub content: HashMap<String, String>,
}

bitflags! {
    pub struct WriteFlag: u32 {
        const NONE      = 0;
        const APPEND    = 1 << 0;
        const CREATE    = 1 << 1;
        const EXCLUSIVE = 1 << 2;
        const TRUNCATE  = 1 << 3;
        const SYNC      = 1 << 4;
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AgfsError {
    #[error("not found: {0}")] NotFound(String),
    #[error("permission denied: {0}")] PermissionDenied(String),
    #[error("already exists: {0}")] AlreadyExists(String),
    #[error("not supported")] NotSupported,
    #[error("invalid argument: {0}")] InvalidArgument(String),
    #[error("io error: {0}")] Io(#[from] std::io::Error),
    #[error("internal: {0}")] Internal(String),
}
```

---

## 模块约定

- **异步策略**：`FileSystem` trait 本身是同步的（Go 原版也是同步的）。异步发生在 HTTP handler 层（axum）。如果某个 plugin 需要异步操作，使用 `tokio::runtime::Handle::current().block_on(...)` 在同步 trait 方法内桥接。
- **错误处理**：所有公开方法返回 `Result<T, AgfsError>`，不要使用 panic
- **线程安全**：所有实现必须是 `Send + Sync`

---

## 与其他模块的依赖

- 被依赖：`agfs-server`（使用所有 trait 和类型）
- 被依赖：`agfs-fuse`（使用 SDK Client 和类型）

---

## Go→Rust 转换注意事项

1. **Go interface 隐式 vs Rust trait 显式**：Go 的 plugin 只要有对应方法就自动实现 FileSystem；Rust 必须写 `impl FileSystem for MyPlugin { ... }`
2. **Go `io.ReadCloser` → Rust `Box<dyn Read + Send>`**：Go 的 Open 返回 ReadCloser，Rust 用 `Box<dyn Read + Send>`，关闭由 Drop 处理
3. **`map[string]interface{}` config → `HashMap<String, serde_json::Value>`**
4. **FileHandle**：Go 版本有 `filesystem.FileHandle` 接口用于有状态操作；Rust 版本在 `agfs-server` 的 MountableFS 层管理 handle ID 映射
