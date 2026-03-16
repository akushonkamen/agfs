# Phase 2: MountableFS 与 HTTP 层 - 讨论频道

> 双向讨论频道，记录技术决策和问题

## 设计决策

### MountableFS 设计
- 使用 `radix_trie::Trie<String, Arc<MountPoint>>` 存储挂载点
- 路径匹配使用最长前缀匹配
- 全局 handle ID 使用 `AtomicI64::new(1)`
- symlink 表使用 `DashMap<String, String>`

### HTTP handler 设计
- 使用 `axum` 框架
- 异步 handler 内部用 `tokio::task::spawn_blocking` 调用同步 FileSystem 方法
- 错误响应统一为 JSON 格式

## 待讨论问题
（暂无）
