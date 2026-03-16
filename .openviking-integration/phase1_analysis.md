# Phase 1 分析报告：OpenViking AGFS 集成点

## 当前状态

### 目录结构
```
/home/yalun/Dev/OpenViking/third_party/agfs/  → 就是当前 CtxFS 项目
├── src/                    ← Rust workspace
│   ├── Cargo.toml
│   ├── sdk/               ← ctxfs-sdk
│   ├── server/            ← ctxfs-server
│   ├── fuse/              ← ctxfs-fuse
│   ├── python-sdk/        ← Python SDK (实际位置)
│   ├── shell/             ← agfs-shell
│   └── target/
│       └── release/
│           └── ctxfs-server ← 二进制文件存在
└── (其他项目文件)
```

### OpenViking 期望的路径

OpenViking 的 `pyproject.toml` 引用：
```toml
pyctxfs = { path = "third_party/agfs/python-sdk" }
```

但实际位置是：`third_party/agfs/src/python-sdk/`

### OpenViking 期望的构建产物

OpenViking 的 `setup.py` 期望：
```
bin/agfs-server (或 .exe)
lib/libagfsbinding.so (或 .dylib, .dll)
```

当前 CtxFS 生成：
```
src/target/release/ctxfs-server
src/target/release/ctxfs-fuse
```

## 问题分析

1. **Python SDK 路径不匹配**：
   - 期望: `third_party/agfs/python-sdk/`
   - 实际: `third_party/agfs/src/python-sdk/`

2. **二进制文件名称不匹配**：
   - 期望: `agfs-server`
   - 实际: `ctxfs-server`

3. **库文件缺失**：
   - 期望: `libagfsbinding.so/dylib/dll` (Python 绑定)
   - 实际: CtxFS 没有这个库 (使用纯 HTTP 客户端)

## 解决方案

### 选项 A: 符号链接方案 (推荐)
```bash
cd /home/yalun/Dev/OpenViking/third_party/agfs
ln -s src/python-sdk python-sdk
ln -s src/target/release/ctxfs-server bin/agfs-server
```

### 选项 B: 修改 OpenViking 配置
- 更新 `pyproject.toml` 路径
- 更新 `setup.py` 构建逻辑
- 添加 CtxFS 特定的集成脚本

### 选项 C: 创建兼容层
- 创建包装脚本 `agfs-server` → `ctxfs-server`
- 创建 python-sdk 兼容接口

## 推荐方案

**选项 A + 部分选项 B**：
1. 创建符号链接解决路径问题
2. 更新 OpenViking 文档说明 CtxFS 变更
3. 保持向后兼容性
