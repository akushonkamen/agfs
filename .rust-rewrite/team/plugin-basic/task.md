# Phase 3: 基础 Plugins（无外部依赖）- 任务清单

Teammate: `plugin-basic-engineer`

## 任务列表

### 1. devfs（设备文件系统）
- [ ] null 设备（/dev/null，丢弃所有写入，读取返回 EOF）
- [ ] zero 设备（/dev/zero，读取返回无限零）
- [ ] random 设备（/dev/random，读取返回随机数据）
- [ ] urandom 设备（/dev/urandom，读取返回随机数据，非阻塞）

### 2. memfs（内存文件系统）
- [ ] 基于内存的文件存储
- [ ] 目录支持
- [ ] 文件 CRUD 操作
- [ ] 并发安全（Arc<RwLock<HashMap>>）

### 3. hellofs（示例 plugin）
- [ ] 简单的只读文件系统
- [ ] 返回 "Hello, World!" 内容

### 4. heartbeatfs（心跳文件系统）
- [ ] 30s 超时自动清理
- [ ] touch 更新时间戳
- [ ] 返回存活 agent 列表

### 5. serverinfofs（服务信息）
- [ ] 对接 TrafficMonitor
- [ ] 返回服务器统计信息

### 6. kvfs（KV 存储）
- [ ] 内存 HashMap 实现
- [ ] set/get/delete 操作
- [ ] 列表操作

## Go 参考
- `agfs-server/pkg/plugins/devfs/`
- `agfs-server/pkg/plugins/memfs/`
- `agfs-server/pkg/plugins/hellofs/`
- `agfs-server/pkg/plugins/heartbeatfs/`
- `agfs-server/pkg/plugins/serverinfofs/`
- `agfs-server/pkg/plugins/kvfs/`

## 验收标准
1. 所有 plugin unit tests 通过
2. 可通过 agfs-shell 正常使用
3. `cargo test --package agfs-server` 通过
4. `cargo clippy --package agfs-server -- -D warnings` 通过
