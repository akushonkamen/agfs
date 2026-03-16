# 模块完成报告 · {module-name}

> **完成时间**：{date}
> **负责 Teammate**：{teammate-name}（已解散）
> **所属 Phase**：Phase {n}
> **Leader 审核**：rewrite-lead

---

## 📦 模块概述

**Go 原始位置**：`go-src/{path}/`
**Rust 实现位置**：`rust-src/src/{module}/`
**代码行数（Go）**：{n} 行
**代码行数（Rust）**：{n} 行

---

## ✅ 完成任务清单

| TASK | 描述 | Commit |
|------|------|--------|
| TASK-001 | ... | abc1234 |

---

## 🧪 测试验收

| 测试类型 | 测试数量 | 通过率 |
|----------|----------|--------|
| Unit Tests | {n} | 100% |
| Integration Tests | {n} | 100% |

```
cargo test 输出摘要：
test result: ok. {n} passed; 0 failed; 0 ignored
```

---

## 🔍 与 Go 原始行为差异记录

> 如有已知差异（含修复的 Bug），在此记录

| 差异 | Go 行为 | Rust 行为 | 原因 |
|------|---------|-----------|------|
| 无   | -       | -         | -    |

---

## 📝 架构决策（ADR 摘要）

从 `discussion.md` 中提取的关键决策：

1. **ADR-001**：{决策内容}
   - 原因：{原因}

---

## ⚠️ 遗留风险

- 无

---

## 🤝 交接说明

下一个依赖本模块的 Teammate 需要知道：

- 公开 trait/struct：{列表}
- 注意事项：{如有}
