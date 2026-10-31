# NovelDownloader

`noveldownloader` 是一个 Rust-first 的小说下载与在线阅读服务。仓库已移除原 Python Web、Python Telegram Bot 与 Python 依赖，主服务由 **Rust + Axum** 提供；Cloudflare Worker 目录作为已部署的签名、加密章节解析、Durable Object 队列与 Telegram 兼容后端保留。

> ⚠️ **免责声明**
> 本项目仅用于学习、研究、个人归档和技术交流。不得用于商业分发、批量爬取、绕过访问控制、侵犯著作权或其他违法用途。小说内容及相关权利归原作者和对应平台所有。使用者应遵守所在地法律、平台规则和版权规则，并自行承担使用本项目产生的一切风险与责任。

## 架构

```text
浏览器 / Telegram / API Client
           │
           ▼
  Rust Server (Axum, :8080)
  ├── MD3 在线阅读页 /read
  ├── API 兼容网关
  ├── 书库与任务接口
  └── HTTP / Docker 部署
           │ WORKER_UPSTREAM_URL
           ▼
Cloudflare Worker（可选但推荐）
  ├── 番茄 APP 请求签名
  ├── 设备注册、keyinfo、章节解密
  ├── Durable Object 下载队列
  ├── KV 状态 / Telegram Bot
  └── txt / html / epub / zip 导出
```

Rust 服务默认以兼容网关模式转发请求到现有 Worker，因此已经上线的签名、正文解密、下载队列、导出与 Telegram 功能不会因迁移中断。这样可以先用 Rust 承担服务器、反向代理、阅读页、书库和后续持久化能力，再逐步将签名算法和加密实现原生迁移为 Rust 模块。

## 已提供的服务接口

| 路径 | 功能 |
|---|---|
| `GET /` | 服务状态与端点清单 |
| `GET /health` | 健康检查、上游/会话配置状态 |
| `GET /read` | Material Design 3 在线阅读页 |
| `GET /api/search?key=` | 搜索兼容接口 |
| `GET /api/detail?book_id=` | 书籍详情 |
| `GET /api/directory?book_id=` | 章节目录 |
| `GET /api/content?item_id=` | 单章正文 |
| `GET /api/download` | 分段文本下载 |
| `GET /api/export` | txt / html / epub 导出 |
| `GET /api/export/zip` | ZIP 导出 |
| `GET/POST /api/library` | 内存书库接口（后续可替换为 SQLite/Postgres） |
| `GET/POST /api/jobs` | 下载任务接口（后续可连接 Redis/数据库队列） |
| `POST /tg/webhook` | Telegram Webhook 兼容转发 |

## 快速部署

### Docker Compose（推荐）

```bash
git clone https://github.com/a8219/noveldownloader.git
cd noveldownloader
cp .env.example .env
# 按需修改 .env 中的 WORKER_UPSTREAM_URL
docker compose up -d --build
```

服务启动后访问：

```text
http://127.0.0.1:8080/read
http://127.0.0.1:8080/health
```

### 原生 Rust

要求：Rust 1.82+。

```bash
cp .env.example .env
cargo run --release
```

默认监听 `0.0.0.0:8080`。可通过 `BIND_ADDR` 修改，例如：

```bash
BIND_ADDR=127.0.0.1:9000 cargo run --release
```

## 环境变量

| 变量 | 默认值 | 说明 |
|---|---|---|
| `BIND_ADDR` | `0.0.0.0:8080` | Rust HTTP 监听地址 |
| `RUST_LOG` | `info,tower_http=info` | 日志过滤规则 |
| `REQUEST_TIMEOUT_SECS` | `45` | 上游 HTTP 请求超时秒数 |
| `WORKER_UPSTREAM_URL` | 空 | 推荐填写已部署 Cloudflare Worker 的地址 |
| `FANQIE_COOKIE` | 空 | 可选的用户提供会话 Cookie；严禁提交进仓库 |

建议使用当前已部署 Worker：

```env
WORKER_UPSTREAM_URL=https://fanqie-downloader.3942369144a.workers.dev
```

## Cloudflare Worker

`workers/` 保留了当前 Worker 源码和部署说明。它负责目前最难且已经稳定运行的部分：请求签名、设备恢复、章节解密、Durable Object、KV 与 Telegram Bot。

部署说明见：[workers/README.md](workers/README.md)。

## 搜索说明

目标站点对自动化搜索请求可能触发风控或返回空响应。服务不会尝试绕过滑块验证或伪造验证结果。对于需要登录态的合法访问，请由用户通过正常浏览器流程获得自己的会话，并通过安全的部署环境配置相应 Secret/Cookie；不要将凭据提交到 GitHub、写入前端或打印到日志。

## 迁移说明

本次仓库重构：

- 删除了 Python 的 `src/`、`tgbot.py`、`requirements.txt`、旧 Python Docker/模板/静态文件；
- 增加 Rust Axum 服务、Docker 镜像、Compose 编排、MD3 阅读页与 API 兼容网关；
- 保留 `workers/`，以避免破坏已经工作的 Cloudflare 签名与下载能力；
- 仓库从 `fanqiedownloader` 重命名为 **`noveldownloader`**。

## License

[AGPL-3.0](LICENSE)