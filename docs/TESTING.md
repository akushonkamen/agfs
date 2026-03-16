# CtxFS 测试指南

## 测试结构（Rust 标准做法）

```
src/
├── server/
│   ├── src/                    # 源代码（单元测试在模块内 #[cfg(test)]）
│   └── tests/                  # 集成测试
│       └── integration_test.rs
├── sdk/
│   ├── src/                    # 源代码（单元测试在模块内）
│   └── tests/                  # SDK 集成测试
└── fuse/
    ├── src/                    # 源代码（单元测试在模块内）
    └── tests/                  # FUSE 集成测试

scripts/
├── run-all-tests.sh            # 运行所有测试
└── run-benchmarks.sh          # 性能基准测试

src/shell/tests/                # Python 单元测试
```

## 运行测试

### 快捷方式

```bash
# 运行所有测试
./ctest.sh

# 运行性能测试
./bench.sh

# 或直接使用脚本
bash scripts/run-all-tests.sh
bash scripts/run-benchmarks.sh
```

### Cargo 命令

```bash
cd src

# 所有单元测试
cargo test --workspace --lib

# 所有集成测试
cargo test --workspace --test integration_test -- --ignored

# 特定 crate 测试
cargo test -p ctxfs-server --lib
cargo test -p ctxfs-sdk --lib
```

### Python 测试

```bash
cd src/shell
uv run pytest tests/ -v
```

## 测试类型

| 类型 | 位置 | 命令 |
|------|------|------|
| 单元测试 | `src/**/src/*.rs` | `cargo test --lib` |
| 集成测试 | `src/**/tests/*.rs` | `cargo test --test *` |
| Python 测试 | `src/shell/tests/*.py` | `pytest tests/` |
| 性能测试 | `scripts/run-benchmarks.sh` | `./bench.sh` |

## 添加新测试

### Rust 单元测试

在源文件中添加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // 测试代码
    }
}
```

### Rust 集成测试

在对应 crate 的 `tests/` 目录添加 `.rs` 文件。

### 性能测试

编辑 `scripts/run-benchmarks.sh` 添加新的测试场景。
