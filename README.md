# NovelDownloader

一个自成一体的小说下载与在线阅读服务器，使用 **Rust + Axum** 原生实现，无需外部 Worker 或代理。

> ⚠️ **免责声明**
>
> 本项目仅供个人学习、研究和技术交流使用。使用本项目即表示您已阅读并同意以下条款：
>
> 1. **版权声明**：所有小说内容的版权归原作者和对应平台所有。本项目不存储、不分发任何小说内容，仅提供技术接口用于个人离线阅读。
> 2. **使用限制**：禁止将本项目用于商业用途、批量爬取、数据倒卖、绕过付费墙或其他侵犯他人权益的行为。
> 3. **风险承担**：使用者应自行承担使用本项目产生的一切风险和后果，包括但不限于账号封禁、IP 封锁、法律纠纷等。本项目作者不对任何因使用本服务造成的直接或间接损失负责。
> 4. **合规使用**：请遵守您所在地的法律法规和平台服务条款。若您不同意以上条款，请立即停止使用并删除本项目。
> 5. **无担保**：本项目按"现状"提供，不提供任何明示或暗示的担保。作者不保证项目的可用性、准确性或完整性。

## 功能

### 原生实现（无需 Worker 或代理）
- **请求签名**：Argus + Ladon 签名（MD5、SHA-512、Protobuf、AES-CBC）
- **设备注册**：自动注册匿名设备，获取 device_id / install_id
- **密钥注册**：通过 `/reading/crypt/registerkey` 获取章节解密密钥
- **章节解密**：AES-CBC 解密 + 字体映射解码（362 个 PUA 字符）
- **HTML 清洗**：标签剥离、实体解码、换行规范化

### API 接口
| 路径 | 功能 |
|---|---|
| `GET /` | 服务状态与端点清单 |
| `GET /health` | 健康检查 |
| `GET /read` | Material Design 3 在线阅读页（含搜索） |
| `GET /api/search?key=` | 书名搜索（原生 APP 接口） |
| `GET /api/detail?book_id=` | 书籍详情 |
| `GET /api/directory?book_id=` | 章节目录 |
| `GET /api/content?item_id=` | 单章正文（签名+解密+字体解码） |
| `GET /api/download` | 分段文本下载 |
| `GET /api/export` | 导出 txt / html / epub |
| `GET /api/export/zip` | ZIP 打包导出 |
| `GET /api/library` | 书库管理 |
| `GET /api/jobs` | 下载任务 |
| `POST /tg/webhook` | Telegram Webhook |

### 在线阅读（MD3）
- Material Design 3 色彩系统（浅色/深色主题）
- 书名搜索栏
- 书籍详情卡片（封面、作者、简介）
- 章节目录列表
- 正文阅读（字号调节、上下章导航）

### 性能优化
- `lto = true`、`codegen-units = 1`、`opt-level = 3`、`strip = true`、`panic = "abort"`
- 纯 Rust 加密实现（无 JS 运行时开销）
- 零拷贝字体映射（LazyLock + HashMap）
- 连接池复用（reqwest + rustls）

## 架构

```text
浏览器 / API Client
       │
       ▼
  Rust Server (Axum, :8080)
  ├── MD3 在线阅读页 /read
  ├── API 路由层
  ├── 搜索模块（直连 APP 接口）
  ├── 签名模块（Argus + Ladon）
  ├── 设备注册 + 密钥注册
  ├── 章节解密 + 字体解码
  ├── 导出模块（txt/html/zip）
  └── Telegram Bot（可选）
```

## 部署

### Docker Compose
```bash
git clone https://github.com/a8219/noveldownloader.git
cd noveldownloader
cp .env.example .env
docker compose up -d --build
```

### 原生 Rust
```bash
cp .env.example .env
cargo run --release
```

访问：`http://127.0.0.1:8080/read`

## 环境变量

| 变量 | 默认值 | 说明 |
|---|---|---|
| `BIND_ADDR` | `0.0.0.0:8080` | 监听地址 |
| `RUST_LOG` | `info,tower_http=info` | 日志级别 |
| `REQUEST_TIMEOUT_SECS` | `45` | 请求超时 |
| `FANQIE_COOKIE` | 空 | 可选会话 Cookie |
| `FANQIE_PROXY_API` | 空 | 可选搜索代理 |
| `TG_BOT_TOKEN` | 空 | Telegram Bot |
| `FANQIE_ONLINE_READ` | `1` | 在线阅读开关 |

## 模块结构

```text
src/
├── main.rs          # 入口
├── lib.rs           # 模块声明
├── config.rs        # 环境变量配置
├── server.rs        # 服务器状态 + 启动
├── read_page.rs     # MD3 阅读页
├── font_map.rs      # 字体映射（362 条）
├── search.rs        # 搜索模块
├── export.rs        # 导出（txt/html/zip）
├── telegram.rs      # Telegram Bot
├── library.rs       # 书库
├── jobs.rs          # 任务队列
├── api/
│   ├── routes.rs    # 路由定义
│   └── handlers.rs  # 请求处理
└── crypto/
    ├── mod.rs
    ├── md5.rs       # 纯 Rust MD5
    ├── sign.rs      # Argus + Ladon 签名
    ├── device.rs    # 设备/密钥注册
    ├── chapter.rs   # 章节获取+解密
    ├── ttencrypt.rs # TT 加密
    └── protobuf.rs  # Protobuf 写入器
```

## Cloudflare Worker

`workers/` 目录保留了已部署的 Worker 源码，作为可选的兼容后端。详见 [workers/README.md](workers/README.md)。

## License

[AGPL-3.0](LICENSE)
