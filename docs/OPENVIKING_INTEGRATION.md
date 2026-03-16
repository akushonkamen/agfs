# AGFS for OpenViking

AGFS Rust 版本已准备就绪，可用于 OpenViking 项目。

## 快速启动

```bash
cd /home/yalun/Dev/agfs
./start-agfs.sh
```

服务器将在 `http://localhost:8080` 启动。

## 验证服务

```bash
# 健康检查
curl http://localhost:8080/api/v1/health

# 查看已加载的插件
curl http://localhost:8080/api/v1/plugins

# 测试 memfs
curl -X POST "http://localhost:8080/api/v1/files?path=/memfs/test.txt"
echo "hello" | curl -X PUT "http://localhost:8080/api/v1/files?path=/memfs/test.txt" --data-binary @-
curl "http://localhost:8080/api/v1/files?path=/memfs/test.txt"
```

## OpenViking 配置

OpenViking 的 AGFS 配置 (`~/.openviking/ov.conf`):

```json
{
  "storage": {
    "agfs": {
      "url": "http://localhost:8080",
      "mode": "http-client"
    }
  }
}
```

## 可用插件

| 插件 | 路径 | 说明 |
|------|------|------|
| memfs | `/memfs` | 内存文件系统 |
| localfs | `/local` | 本地目录映射到 `/tmp/agfs-local` |

## 停止服务

```bash
./stop-agfs.sh
```

## systemd 服务（可选）

```bash
# 安装服务
sudo cp agfs-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable agfs-server
sudo systemctl start agfs-server

# 查看状态
sudo systemctl status agfs-server
```

## API 端点

| 端点 | 方法 | 功能 |
|------|------|------|
| `/api/v1/health` | GET | 健康检查 |
| `/api/v1/capabilities` | GET | 服务能力 |
| `/api/v1/plugins` | GET | 插件列表 |
| `/api/v1/files` | GET/POST/PUT | 文件操作 |
| `/api/v1/directories` | GET/POST | 目录操作 |
| `/api/v1/stat` | GET | 文件元信息 |

## 技术栈

- **语言**: Rust
- **HTTP 框架**: axum
- **异步运行时**: tokio
- **源码**: `/home/yalun/Dev/agfs/rust-src/`
