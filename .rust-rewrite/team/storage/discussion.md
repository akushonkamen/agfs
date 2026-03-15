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

## 实现进度记录

### 2025-03-15 - Phase 5 完成 ✅

**localfs 实现完成：**
- 完整 FileSystem trait 实现
- Truncater 扩展：支持文件截断操作
- Symlinker 扩展：支持创建和读取符号链接
- Streamer 扩展：支持 64KB 分块的流式读取
- 使用 `std::os::unix::fs::MetadataExt` 获取文件模式

**s3fs 基础框架：**
- S3FS 结构和 S3Metadata 缓存
- 为 aws-sdk-s3 集成预留接口
- 元数据缓存使用 Arc<RwLock<HashMap>>

**sqlfs 内存实现：**
- 使用 HashMap 作为临时存储
- 完整 FileSystem trait 实现
- 为 sqlx 集成预留接口

**sqlfs2 Plan 9 风格接口：**
- Session 管理框架
- 路径解析：支持 `/dbName/tableName/{ctl,schema,count,sid/query,result,error}` 结构
- 基础测试通过

**技术亮点：**
- 跨平台兼容：Unix/Windows 分别处理符号链接和文件权限
- 线程安全：使用 Arc 和 RwLock 确保并发安全
- 错误处理：统一的 AgfsError 类型
- 测试覆盖：所有插件都有单元测试

## 待讨论问题
（暂无，Phase 5 已完成）
