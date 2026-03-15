# Phase 3: 基础 Plugins - 讨论频道

## 设计决策

### devfs 设计
- 每个设备作为一个独立的文件
- 读取返回特殊数据（null: EOF, zero: 无限零, random: 随机字节）
- 写入被忽略（null）或返回成功

### memfs 设计
- 使用 `Arc<RwLock<HashMap<String, Vec<u8>>>>` 存储文件
- 目录使用单独的 HashSet 管理
- 支持 Create, Mkdir, Remove, Read, Write, ReadDir, Stat

### heartbeatfs 设计
- 使用 `DashMap<String, Instant>` 存储心跳时间
- 后台任务定期清理过期条目（30s）
- 实现现在已由 TrafficMonitor 提供，简化实现

## 待讨论问题
（暂无）
