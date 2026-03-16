# CtxFS 测试文档

本目录包含 CtxFS 项目的所有测试脚本和测试数据。

## 目录结构

```
tests/
├── run-all-tests.sh          # 一键运行所有测试
├── unit/                     # 单元测试（位于各 crate 的 tests/ 目录）
├── integration/              # 集成测试
├── performance/              # 性能测试
│   └── run-benchmarks.sh     # 性能基准测试脚本
├── fixtures/                 # 测试数据文件
└── test-config.yaml          # 测试配置
```

## 快速开始

### 运行所有测试

```bash
./tests/run-all-tests.sh
```

### 只运行特定类型测试

```bash
# 只运行单元测试
./tests/run-all-tests.sh --unit

# 只运行集成测试
./tests/run-all-tests.sh --integration

# 只运行性能测试
./tests/run-all-tests.sh --perf

# 只运行 Rust 测试
./tests/run-all-tests.sh --rust

# 只运行 Python 测试
./tests/run-all-tests.sh --python
```

### 详细输出

```bash
./tests/run-all-tests.sh --verbose
```

## 测试类型

### 1. 单元测试

各 crate 内的单元测试，使用 `cargo test` 运行：

```bash
# 运行所有单元测试
cargo test --workspace --lib

# 运行特定 crate 的单元测试
cargo test -p ctxfs-sdk --lib
cargo test -p ctxfs-server --lib
```

### 2. 集成测试

端到端的 API 测试，需要服务运行：

```bash
# 先启动服务
./start-agfs.sh

# 运行集成测试
cargo test --workspace --test integration_test -- --ignored
```

### 3. 性能测试

性能基准测试，测量吞吐量和延迟：

```bash
# 先启动服务
./start-agfs.sh

# 运行性能测试
./tests/performance/run-benchmarks.sh
```

### 4. Python 测试

shell 组件的 Python 单元测试：

```bash
cd src/shell
uv run pytest tests/ -v
```

## 测试数据

测试数据文件位于 `tests/fixtures/` 目录：

- `1kb.bin` - 1KB 测试文件
- `10kb.bin` - 10KB 测试文件
- `100kb.bin` - 100KB 测试文件
- `1mb.bin` - 1MB 测试文件
- `10mb.bin` - 10MB 测试文件

## CI/CD

这些测试脚本也会在 CI/CD 中自动运行：

- GitHub Actions: `.github/workflows/test.yml`
- 每次提交都会运行完整测试套件

## 添加新测试

### Rust 单元测试

在对应 crate 的 `src/` 目录下添加 `#[cfg(test)]` 模块或在 `tests/` 目录添加测试文件。

### Rust 集成测试

在 `src/server/tests/integration_test.rs` 中添加新的测试函数，标记为 `#[ignore]`。

### 性能测试

编辑 `tests/performance/run-benchmarks.sh` 添加新的测试场景。

## 故障排查

### 测试失败

1. 检查服务是否运行：`curl http://localhost:8080/api/v1/health`
2. 检查端口占用：`lsof -i :8080`
3. 查看服务日志：`tail -f /tmp/ctxfs-server.log`

### 清理测试数据

```bash
# 停止服务
./stop-agfs.sh

# 清理临时文件
rm -rf /tmp/ctxfs-*
```
