# OpenViking AGFS 集成 · Phase 计划

**目标**: 将当前 CtxFS (Rust) 项目集成到 OpenViking 的 `third_party/agfs/`

**源项目**: `/home/yalun/Dev/agfs/` (CtxFS, Phase 0-8 已完成)
**目标项目**: `/home/yalun/Dev/OpenViking/third_party/agfs/`

---

## Phase 0: 验证当前项目状态
**Teammate**: `qa-engineer`
**任务**:
- 运行完整测试套件: `cargo test --workspace`
- 编译 release 版本: `cargo build --release`
- 验证所有 18 个 plugins 功能正常
- 生成测试报告

**验收**: 所有测试通过，编译成功

---

## Phase 1: 分析 OpenViking AGFS 集成点
**Teammate**: `integration-analyzer`
**任务**:
- 分析 OpenViking 如何使用 AGFS
- 检查 `third_party/agfs/` 当前状态
- 识别 AGFS API 依赖关系
- 确定兼容性要求

**验收**: 集成点分析报告

---

## Phase 2: 准备迁移
**Teammate**: `migration-prep`
**任务**:
- 备份 OpenViking 当前 AGFS
- 清理 `third_party/agfs/` 目录
- 准备迁移脚本
- 更新 .gitignore

**验收**: 迁移环境准备完成

---

## Phase 3: 执行迁移
**Teammate**: `migration-exec`
**任务**:
- 复制 CtxFS 源码到 OpenViking
- 更新 Cargo.toml 路径
- 更新 Python SDK 路径
- 保持目录结构兼容

**验收**: 代码迁移完成

---

## Phase 4: 修复集成问题
**Teammate**: `integration-fixer`
**任务**:
- 修复编译错误
- 更新 API 兼容性
- 调整依赖路径
- 处理命名变更 (AGFS → CtxFS)

**验收**: OpenViking 可编译

---

## Phase 5: 验证集成
**Teammate**: `integration-qa`
**任务**:
- 运行 OpenViking 测试
- 验证 AGFS 功能正常
- 性能回归测试
- 生成集成报告

**验收**: 所有测试通过

---

## 团队管理

**同步限制**: 同时只有 1-2 个 teammate 活跃
**Phase 顺序**: 0 → 1 → 2 → 3 → 4 → 5
**Git 规范**: 每个 Phase 完成后提交
