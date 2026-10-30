# 番茄小说下载器（Cloudflare Workers 版）

基于 Cloudflare Workers 的番茄小说下载 / 在线阅读工具，附带 Telegram 机器人管理后台。

## 功能

### 下载
- 输入番茄小说书籍链接或 ID，自动抓取目录并下载全本
- 输出格式：TXT / HTML / EPUB，支持打包 ZIP
- 多章并发 + 断点续传，按章节顺序合并，自动补齐缺章
- 章节正文自动解码 PUA 字体混淆
- 支持增量更新（只下载新增章节）

### 接口
| 接口 | 说明 |
|---|---|
| `GET /api/search?key=` | 搜索书籍 |
| `GET /api/detail?book_id=` | 书籍详情 |
| `GET /api/directory?book_id=` | 章节目录 |
| `GET /api/content?item_id=` | 单章正文 |
| `GET /api/download?book_id=&start=&count=` | 分批下载正文 |
| `GET /api/export?book_id=&format=txt\|html\|epub` | 导出整本 |
| `GET /api/export/zip?book_id=&format=txt\|html` | 导出 ZIP |
| `GET /api/update?book_id=` | 检查更新 |
| `GET /api/config` | 运行配置 |
| `GET /read` | 在线阅读页（需开启开关） |

### Telegram 机器人
- 私聊发送书籍链接 / ID → 自动下载并回传文件
- 实时进度（编辑同一条消息）
- 账号系统：`/signup` 注册、`/login` 登录
- 管理员后台 `/admin`：用户列表、封禁 / 解封、每日下载上限、欢迎语、公告、登录日志
- 管理员不受每日上限限制
- 异常登录告警

### 在线阅读
- 访问 `GET /read`，输入书籍链接或 ID 直接在线阅读
- 目录浏览、上一章 / 下一章、字号调节、日间 / 夜间主题
- 由环境变量 `FANQIE_ONLINE_READ` 控制：`1` 启用，`0` 禁用（默认 `0`）

## 部署

### 1. 准备
- Cloudflare 账号
- Node.js 18+ 与 `npx wrangler`