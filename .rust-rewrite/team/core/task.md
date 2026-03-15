# Phase 1: 核心类型与 FileSystem trait - 任务清单

Teammate: `core-engineer`

## 任务列表

### 1. FileSystem trait
- [x] 实现 `FileSystem` trait（对应 Go `filesystem.FileSystem` interface）
- [x] 所有方法签名与 Go 版本保持一致
- [x] 返回类型使用 `Result<T, AgfsError>`

### 2. 扩展 trait
- [x] `Streamer` trait - 用于 streamfs
- [x] `Toucher` trait - 用于 heartbeatfs
- [x] `Symlinker` trait - 符号链接支持
- [x] `Truncater` trait - 文件截断

### 3. ServicePlugin trait
- [x] 实现 `ServicePlugin` trait
- [x] `ConfigParameter` struct

### 4. 类型完善
- [x] `WriteFlag` 使用 bitflags
- [x] `OpenFlag` 完善
- [x] `FileInfo` 完善（添加 `is_symlink` 字段）
- [x] `StreamReader` trait

### 5. SDK Client 实现
- [x] 实现所有 API 端点方法（files, directories, stat, rename, chmod, touch, truncate, symlink, readlink, grep, digest, handles, capabilities, health, plugins）
- [x] 单元测试

### 6. 错误处理完善
- [x] 添加 `AlreadyExists`, `NotSupported` 错误变体

## Go 参考
- `agfs-server/pkg/filesystem/filesystem.go` - FileSystem interface
- `agfs-server/pkg/plugin/plugin.go` - ServicePlugin interface
- `agfs-server/pkg/filesystem/errors.go` - 错误类型
- `agfs-sdk/go/types.go` - SDK 类型
- `agfs-sdk/go/client.go` - SDK Client 实现

## 验收标准
1. ✅ 所有 trait 和类型可编译
2. ✅ SDK client 有 unit tests
3. ✅ `cargo test --package agfs-sdk` 通过
4. ✅ `cargo clippy --package agfs-sdk -- -D warnings` 通过

## 完成总结

已实现以下文件：
- `rust-src/agfs-sdk/src/filesystem.rs` - FileSystem trait 及扩展 trait (Streamer, Toucher, Symlinker, Truncater)
- `rust-src/agfs-sdk/src/plugin.rs` - ServicePlugin trait 及相关类型
- `rust-src/agfs-sdk/src/types.rs` - 所有共享类型，包括 WriteFlag/OpenFlag bitflags, FileInfo, MetaData, API 请求/响应类型
- `rust-src/agfs-sdk/src/error.rs` - 完善的 AgfsError 枚举
- `rust-src/agfs-sdk/src/client.rs` - 完整的 HTTP Client 实现，包含所有 API 端点

所有验收标准已达成，任务完成。
