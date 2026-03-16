# fsgrep - 服务器端 Grep 命令

`fsgrep` 是一个新的 agfs-shell 命令，它调用 AGFS 服务器端的 grep 实现。不同于本地的 `grep` 命令，`fsgrep` 利用各个文件系统的自定义搜索功能。

## 主要特性

### VectorFS 语义搜索
当路径指向 `/vectorfs` 时，`fsgrep` 使用向量嵌入进行语义搜索：
- 搜索词不需要精确匹配
- 基于语义相似度返回结果
- 自动按相关性排序

### 其他文件系统
对于其他文件系统（如 `/local`, `/s3fs`, `/sqlfs`），使用标准正则表达式搜索。

## 语法

```bash
fsgrep [OPTIONS] PATTERN PATH
```

### 选项

- `-r` : 递归搜索（VectorFS 默认开启）
- `-i` : 忽略大小写（仅用于文本搜索）
- `-n` : 显示行号（默认开启）
- `-c` : 仅显示匹配数量
- `-q` : 静默模式（仅返回是否找到匹配）

## 使用示例

### VectorFS 语义搜索

```bash
# 搜索关于容器编排的文档
agfs:/> fsgrep "container orchestration" /vectorfs/test_project/docs
/vectorfs/test_project/docs/kubernetes.txt:1: Kubernetes is a powerful container orchestration platform... [score: 0.892]
/vectorfs/test_project/docs/docker.txt:1: Docker is a platform for developing, shipping, and running... [score: 0.756]

# 搜索基础设施自动化相关内容
agfs:/> fsgrep "infrastructure automation" /vectorfs/test_project/docs
/vectorfs/test_project/docs/terraform.txt:1: Terraform is an infrastructure as code tool... [score: 0.934]
/vectorfs/test_project/docs/kubernetes.txt:1: Kubernetes provides automated deployment... [score: 0.723]

# 仅显示匹配数量
agfs:/> fsgrep -c "cloud deployment" /vectorfs/my_namespace/docs
3

# 检查是否有匹配（用于脚本）
agfs:/> fsgrep -q "database" /vectorfs/project/docs && echo "Found database docs"
Found database docs
```

### 常规文本搜索

```bash
# 在本地文件系统中搜索错误日志
agfs:/> fsgrep "error" /local/tmp/app.log
/local/tmp/app.log:42: [ERROR] Connection failed
/local/tmp/app.log:156: [ERROR] Timeout occurred

# S3 上的日志文件
agfs:/> fsgrep -i "warning" /s3fs/aws/logs/app.log
/s3fs/aws/logs/app.log:23: [WARNING] Low memory

# 递归搜索代码仓库
agfs:/> fsgrep -r "TODO" /sqlfs/tidb/code/
/sqlfs/tidb/code/main.go:15: // TODO: refactor this
/sqlfs/tidb/code/utils.go:89: // TODO: add error handling
```

## 在脚本中使用

```bash
# 检查特定主题的文档是否存在
if fsgrep -q "kubernetes deployment" /vectorfs/docs/production; then
    echo "Found deployment documentation"
else
    echo "Missing deployment docs"
fi

# 统计相关文档数量
count=$(fsgrep -c "microservices" /vectorfs/architecture/docs)
echo "Found $count documents about microservices"

# 提取所有匹配并处理
fsgrep "database migration" /vectorfs/project/docs | while read line; do
    echo "Processing: $line"
done
```

## 与 grep 的区别

| 特性 | grep | fsgrep |
|------|------|--------|
| 执行位置 | 客户端（本地） | 服务器端 |
| VectorFS | 不支持 | 语义搜索 |
| 搜索方式 | 正则表达式 | FS 自定义实现 |
| 性能 | 需要下载文件 | 服务器端处理 |
| 用途 | 文本过滤/管道 | 文件系统搜索 |

## 推荐使用场景

### 使用 `fsgrep`
- ✅ 在 VectorFS 中进行语义搜索
- ✅ 搜索大型文件或目录
- ✅ 利用文件系统的索引功能
- ✅ 跨多个文件搜索

### 使用 `grep`
- ✅ 管道过滤（`cat file | grep pattern`）
- ✅ 本地文本处理
- ✅ 复杂的正则表达式组合
- ✅ 需要 grep 的高级选项（-A, -B, -C 等）

## 技术细节

`fsgrep` 调用 AGFS 的 `/api/v1/grep` 端点：

```json
POST /api/v1/grep
{
  "path": "/vectorfs/namespace/docs",
  "pattern": "search query",
  "recursive": true,
  "case_insensitive": false
}
```

响应格式：

```json
{
  "matches": [
    {
      "file": "/vectorfs/namespace/docs/file.txt",
      "line": 1,
      "content": "matched text content",
      "metadata": {
        "score": 0.892,
        "distance": 0.108
      }
    }
  ],
  "count": 3
}
```

## 常见问题

**Q: fsgrep 和 grep 可以一起使用吗？**
A: 可以！它们互补。用 fsgrep 搜索文件，用 grep 过滤结果：
```bash
fsgrep "kubernetes" /vectorfs/docs | grep deployment
```

**Q: 为什么我的路径不支持 fsgrep？**
A: 只有实现了自定义 grep 的文件系统才支持 fsgrep（如 VectorFS）。其他路径请使用普通 grep。

**Q: VectorFS 的搜索结果是如何排序的？**
A: 按语义相似度排序，最相关的结果在最前面。score 越高越相关（0-1 之间）。

**Q: 如何提高 VectorFS 搜索质量？**
A:
- 使用描述性的查询短语而非单个关键词
- 用自然语言描述你要找的内容
- 例如："容器编排平台" 比 "容器" 效果更好
