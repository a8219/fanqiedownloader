# NovelDownloader

一个自成一体的小说下载与在线阅读服务器，使用 **Rust + Axum** 原生实现。附带 Cloudflare Worker 兼容后端。

> ⚠️ **免责声明**
>
> 本项目仅供个人学习、研究和技术交流使用。使用本项目即表示您已阅读并同意以下条款：
>
> 1. **版权声明**：所有小说内容的版权归原作者和对应平台所有。本项目不存储、不分发任何小说内容，仅提供技术接口用于个人离线阅读。
> 2. **使用限制**：禁止将本项目用于商业用途、批量爬取、数据倒卖或其他侵犯他人权益的行为。
> 3. **风险承担**：使用本项目可能导致账号封禁、IP 封锁或其他平台处罚。使用者应自行承担一切风险和后果。本项目作者不对任何因使用本服务造成的直接或间接损失负责。
> 4. **合规使用**：请遵守您所在地的法律法规和平台服务条款。若您不同意以上条款，请立即停止使用并删除本项目。
> 5. **无担保**：本项目按"现状"提供，不提供任何明示或暗示的担保。作者不保证项目的可用性、准确性或完整性。

## 致谢 / Credits

本项目的部分功能建立在其他开源项目的工作之上，在此感谢以下作者：

| 项目 | 作者 | 许可证 | 贡献内容 |
|---|---|---|---|
| [fanqie-assistant](https://github.com/naiyQAQ/fanqie-assistant) | [naiyQAQ](https://github.com/naiyQAQ) | GPL-3.0 | APP 请求签名（X-Argus/X-Ladon）、匿名设备注册、章节加密解密 |
| [Vue.js](https://github.com/vuejs/core) | Yuxi (Evan) You | MIT | Worker 内打包的 Vue runtime（实际未使用） |
| [fanqienovel-downloader](https://github.com/ying-ck/fanqienovel-downloader) | [ying-ck](https://github.com/ying-ck) | AGPL-3.0 | 番茄公开 API 契约参考（本项目最初衍生自此，后完全重写） |
| [qxqycb/search-novel](https://github.com/qxqycb/search-novel) | [qxqycb](https://github.com/qxqycb) | — | 搜索接口参数参考 |
| [fanqienovel-downloader](https://github.com/lingo34/fanqienovel-downloader) | [lingo34](https://github.com/lingo34) | — | 目录/正文解析思路参考 |

原创部分（MD3 阅读页、字体映射、Telegram Bot、Durable Object 队列、Rust API 层、Docker 部署等）由 [a8219](https://github.com/a8219) 开发。

完整的第三方代码声明与许可详情见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

## 功能

### 原生实现
- **请求签名**：Argus + Ladon 签名（SM3、Simon-128、Speck-128、Protobuf、AES-CBC）
- **设备注册**：自动注册匿名设备，获取 device_id / install_id
- **密钥注册**：通过 `/reading/crypt/registerkey` 获取章节解密密钥
- **章节解密**：AES-CBC 解密 + 字体映射解码（362 个 PUA 字符）
- **HTML 清洗**：标签剥离、实体解码、换行规范化
- **MD3 在线阅读**：搜索、目录、正文、主题切换、字号调节

### API 接口
| 路径 | 功能 |
|---|---|
| `GET /` | 服务状态与端点清单 |
| `GET /health` | 健康检查 |
| `GET /read` | Material Design 3 在线阅读页 |
| `GET /api/search?key=` | 书名搜索 |
| `GET /api/detail?book_id=` | 书籍详情 |
| `GET /api/directory?book_id=` | 章节目录 |
| `GET /api/content?item_id=` | 单章正文 |
| `GET /api/download` | 分段文本下载 |
| `GET /api/export` | 导出 txt / html / epub |
| `GET /api/export/zip` | ZIP 打包导出 |
| `GET /api/library` | 书库管理 |
| `GET /api/jobs` | 下载任务 |
| `POST /tg/webhook` | Telegram Webhook |

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
cargo run --release
```

访问 `http://127.0.0.1:8080/read`

## 环境变量

| 变量 | 默认值 | 说明 |
|---|---|---|
| `BIND_ADDR` | `0.0.0.0:8080` | 监听地址 |
| `RUST_LOG` | `info,tower_http=info` | 日志级别 |
| `REQUEST_TIMEOUT_SECS` | `45` | 请求超时 |
| `FANQIE_COOKIE` | 空 | 可选会话 Cookie |
| `FANQIE_PROXY_API` | 空 | 可选搜索代理 |
| `TG_BOT_TOKEN` | 空 | Telegram Bot Token |
| `FANQIE_ONLINE_READ` | `1` | 在线阅读开关 |

## 许可证

本项目以 [AGPL-3.0](LICENSE) 分发。

选择 AGPL-3.0 是因为 `workers/worker.js` 包含来自 [fanqie-assistant](https://github.com/naiyQAQ/fanqie-assistant)（GPL-3.0）的代码，根据 GPL-3.0 的传染性条款，衍生作品须以兼容许可证分发。AGPL-3.0 与 GPL-3.0 兼容，且额外要求网络服务也开源源码。

详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。