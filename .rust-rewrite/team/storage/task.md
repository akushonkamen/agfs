# Phase 5: 存储 Plugins（外部依赖）- 任务清单

Teammate: `plugin-storage-engineer`

## 任务列表

### 1. localfs（本地文件系统）
- [ ] 挂载本地目录到虚拟路径
- [ ] 实现所有 FileSystem 方法
- [ ] 支持文件 CRUD 操作

### 2. s3fs（S3 对象存储）
- [ ] S3 client 配置
- [ ] 文件上传/下载
- [ ] multipart upload 支持
- [ ] metadata cache

### 3. sqlfs（SQL 数据库文件系统）
- [ ] SQLite/PostgreSQL backend
- [ ] 文件存储为 BLOB
- [ ] 目录结构存储

### 4. sqlfs2（改进版 SQL FS）
- [ ] MySQL/SQLite/TiDB backend
- [ ] Plan 9 风格 session（ctl 文件获取 session ID）
- [ ] query/result 文件

## Go 参考
- `agfs-server/pkg/plugins/localfs/`
- `agfs-server/pkg/plugins/s3fs/`
- `agfs-server/pkg/plugins/sqlfs/`
- `agfs-server/pkg/plugins/sqlfs2/`

## 验收标准
1. localfs 和 sqlfs2 integration tests 通过
2. `cargo test --package agfs-server` 通过
3. `cargo clippy --package agfs-server -- -D warnings` 通过
4. 完成后提交代码
