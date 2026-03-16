# Phase 2: MountableFS 与 HTTP 层 - 任务清单

Teammate: `server-engineer`

## 任务列表

### 1. MountableFS 核心
- [ ] 使用 `radix_trie` 实现路径路由
- [ ] plugin 注册/挂载/卸载
- [ ] 全局 handle ID 管理（AtomicI64）
- [ ] symlink 映射表
- [ ] 实现 FileSystem trait（路由到对应 plugin）

### 2. HTTP handlers（axum）
- [ ] `/api/v1/files` - GET/POST/PUT/DELETE
- [ ] `/api/v1/directories` - GET/POST/DELETE
- [ ] `/api/v1/stat` - GET
- [ ] `/api/v1/rename` - POST
- [ ] `/api/v1/chmod` - POST
- [ ] `/api/v1/touch` - POST
- [ ] `/api/v1/truncate` - POST
- [ ] `/api/v1/symlink` - POST
- [ ] `/api/v1/readlink` - GET
- [ ] `/api/v1/grep` - POST（含 NDJSON 流式输出）
- [ ] `/api/v1/digest` - POST
- [ ] `/api/v1/handles/*` - 有状态 handle 操作
- [ ] `/api/v1/capabilities` - GET
- [ ] `/api/v1/health` - GET
- [ ] `/api/v1/plugins/*` - plugin 管理 API

### 3. 流式读支持
- [ ] chunked transfer encoding
- [ ] `stream=true` 参数支持

### 4. Traffic monitor
- [ ] 使用原子计数器实现

### 5. 配置加载
- [ ] YAML 配置解析
- [ ] plugin 初始化

### 6. 错误处理
- [ ] HTTP 错误响应映射
- [ ] AgfsError → HTTP status code

## Go 参考
- `agfs-server/pkg/mountablefs/mountablefs.go`
- `agfs-server/pkg/handlers/handlers.go`
- `agfs-server/pkg/handlers/handle_handlers.go`
- `agfs-server/pkg/handlers/plugin_handlers.go`
- `agfs-server/pkg/config/config.go`
- `agfs-server/pkg/traffic_monitor.go`

## 验收标准
1. 空 plugin 可挂载
2. 所有 API 端点返回正确格式
3. integration tests 覆盖所有端点
4. `cargo test --package agfs-server` 通过
5. `cargo clippy --package agfs-server -- -D warnings` 通过
