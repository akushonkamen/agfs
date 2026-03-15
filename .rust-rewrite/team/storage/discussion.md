# Phase 5: 存储 Plugins - 讨论频道

## 设计决策

### localfs 设计
- 直接映射到本地文件系统
- 使用 std::fs 进行文件操作
- 需要处理路径边界检查

### s3fs 设计
- 使用 aws-sdk-s3
- multipart upload 使用 S3 的分片上传功能
- metadata cache 使用 Arc<RwLock<HashMap>>

### sqlfs/sqlfs2 设计
- 使用 sqlx 进行数据库操作
- 文件内容存储为 BLOB
- sqlfs2 使用 Plan 9 风格接口

## 待讨论问题
（暂无）
