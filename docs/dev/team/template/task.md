# Task Board · {module-name} 模块

> **负责人**：{teammate-name}
> **创建时间**：{date}
> **所属 Phase**：Phase {n}
> **Leader**：rewrite-lead

---

## 📊 进度概览

| 总任务 | 已完成 | 进行中 | 待开始 | 阻塞 |
|--------|--------|--------|--------|------|
| 0      | 0      | 0      | 0      | 0    |

> ⚠️ 每次更新任务状态后，请同步更新上方统计数字

---

## 🔴 高优先级任务

> Leader 认为阻塞后续模块的关键任务

### TASK-001 · {任务标题}

**状态**：⬜ 待开始 / 🟡 进行中 / ✅ 完成 / 🚫 阻塞

**描述**：
{详细描述}

**对应 Go 文件**：
- `go-src/{path}/file.go`（参考实现）

**Rust 目标文件**：
- `rust-src/{path}/file.rs`

**验收条件**：
- [ ] {具体可验证的条件1}
- [ ] {具体可验证的条件2}
- [ ] 对应 unit test 通过：`cargo test {test_name}`

**依赖**：
- 前置：{TASK-XXX 或 "无"}
- 被依赖：{TASK-XXX 或 "无"}

**完成记录**：
<!-- 完成后填写：✅ 完成于 {日期}，commit: {hash} -->

---

## 🟡 普通任务

### TASK-002 · {任务标题}

**状态**：⬜ 待开始

**描述**：{描述}

**对应 Go 文件**：`go-src/...`

**Rust 目标文件**：`rust-src/...`

**验收条件**：
- [ ] {条件}

**完成记录**：

---

## 🟢 低优先级任务（优化/文档）

### TASK-010 · 添加文档注释

**状态**：⬜ 待开始

**描述**：为所有公开 API 添加 Rust doc comments

**验收条件**：
- [ ] `cargo doc --no-deps` 无警告

---

## 📋 变更记录

| 日期 | 变更内容 | 变更人 |
|------|----------|--------|
|      | 初始任务创建 | rewrite-lead |

---

## 🚨 阻塞问题日志

> 遇到阻塞时在此记录，并通过 mailbox 发送 `[BLOCKER]` 消息给 rewrite-lead

| 时间 | 阻塞描述 | 影响任务 | 解决方案 | 解决时间 |
|------|----------|----------|----------|----------|

---

## ✅ 模块完成 Checklist

完成所有任务后，确认以下项目，然后通过 mailbox 通知 rewrite-lead：

- [ ] 所有 TASK 状态为 ✅
- [ ] `cargo clippy -- -D warnings` 零警告
- [ ] `cargo test` 本模块所有测试通过
- [ ] 代码已 commit，message 规范符合 `[{module}] feat/fix: xxx`
- [ ] discussion.md 中未解决的讨论已全部关闭
- [ ] 已在 discussion.md 底部填写"模块完成确认"

**完成通知模板**（发送给 rewrite-lead 的 mailbox 消息）：
```
[完成通知] {module} 模块所有任务已完成
Phase: {n}
完成任务数: {n}
测试状态: cargo test 全部通过
特别说明: {如有}
请 review 后解散本 Teammate。
```
