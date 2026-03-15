# Phase 0: Workspace 初始化 - 任务清单

Teammate: `infra-engineer`

## 任务列表

- [x] 创建 Cargo workspace（`rust-src/Cargo.toml`）
- [x] 配置所有依赖（见 CLAUDE.md 技术栈表）
- [x] 设置 `rustfmt.toml`、`.clippy.toml`
- [x] 创建 CI（`.github/workflows/rust.yml`）
- [x] 创建 `Dockerfile` 骨架（参考原 Go Dockerfile）
- [x] 建立测试目录结构
- [x] 验收：`cargo build` 通过（允许空实现）

## Go 参考
- `agfs-server/go.mod` - 查看 Go 版本的依赖
- `agfs-server/Dockerfile` - Dockerfile 参考
- `agfs-fuse/go.mod` - FUSE 相关依赖

## 验收标准
1. ✅ `cargo build --workspace` 成功
2. ✅ `cargo fmt --check` 通过
3. ✅ `cargo clippy -- -D warnings` 通过（空实现可能需要一些 allow）
4. ✅ `.github/workflows/rust.yml` 存在且语法正确
5. ✅ `Dockerfile` 存在且可以构建基础镜像

## 完成时间
2025-03-15

## 备注
- 所有空实现模块添加了 `#![allow(missing_docs)]` 以通过 clippy
- 使用 `rustls-tls` 代替 `native-tls` 避免 OpenSSL 依赖
- 测试目录结构已创建：`agfs-sdk/tests/`、`agfs-server/tests/`、`agfs-fuse/tests/`
