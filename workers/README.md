# 番茄小说下载器 — Cloudflare Workers 版

基于 [a8219/fanqiedownloader](https://github.com/a8219/fanqiedownloader) 的 `src/main.py` 迁移。

单文件 `worker.js`（约 26 KB），部署到 Cloudflare Workers 后同时提供 **HTTP API** 和 **Telegram Bot**，发一条链接自动下载全书并回传 TXT。

## 1. 特性

| 能力 | 实现 | 状态 |
|---|---|---|
| 书籍详情 | 网页 `__INITIAL_STATE__.page` | 实测通过 |
| 章节目录 | 网页 `page.itemIds`（全量倒序） | 实测通过 |
| 章节正文 | 网页 `reader.chapterData.content` | 实测通过 |
| 字体反爬解码 | 内联 362 条 PUA 映射 | 362/362 |
| TXT 下载 | 分页聚合 | 通过 |
| Telegram Bot | Webhook + 全自动队列 | 通过 |
| 秒级全本下载 | Durable Objects 链式 alarm | 22 轮 × 1s |
| 分钟级兜底 | Cron + KV 队列 | 通过 |
| 代理 API | 用户配置 / 硬编码池 | 可选 |
| 搜索 | 需代理（网页路由 404） | 受限 |

## 2. 架构

~~~
Telegram -> POST /tg/webhook -> 解析 URL/book_id
                              |
                    hDirectory() GET /page/{id}
                    解析 __INITIAL_STATE__.page.itemIds
                              |
        +---------------------+---------------------+
        v                     v                     v
   FANQIE_DO            FANQIE_KV              都没绑
  (秒级，推荐)         (分钟级兜底)          (单次 30 章)
        |                     |                     |
   链式 alarm            Cron 每分钟          直接抓 30 章
   每轮 45 章            每轮 45 章
        +---------------------+---------------------+
                              v
                    拼全文 -> sendDocument
~~~

绑定优先级（代码自动选择，无需改代码）：

1. `FANQIE_DO` -> Durable Objects 链式 alarm（秒级）
2. 仅 `FANQIE_KV` -> Cron 队列（分钟级）
3. 都没绑 -> 单次 30 章降级

## 3. 数据来源

### 3.1 网页回退（无需签名、无需代理）

| 数据 | 请求 | 字段路径 |
|---|---|---|
| 详情 | `GET https://fanqienovel.com/page/{book_id}` | `__INITIAL_STATE__.page.bookName / author / abstract / status / wordNumber / chapterTotal` |
| 目录 | 同上 | `__INITIAL_STATE__.page.itemIds`（全量章节 id，倒序，代码已 reverse） |
| 正文 | `GET https://fanqienovel.com/reader/{item_id}` | `__INITIAL_STATE__.reader.chapterData.content`（HTML，含 PUA 字符） |

`parseState()` 用括号平衡扫描从 HTML 中截取 JSON，不依赖正则。

### 3.2 代理 API 契约（来自 `src/main.py`）

| 接口 | 参数 | 响应字段 |
|---|---|---|
| `/search` | `key`, `tab_type=3` | `data.search_tabs[tab_type=3].data[].book_data[0]` |
| `/detail` | `book_id` | `data.data.book_name` / `status` |
| `/directory` | `book_id` | `data.lists[].title` / `item_id` |
| `/content` | `tab=小说`, `item_id` | `data.content` |

查找顺序：`FANQIE_PROXY_API` 环境变量 -> KV `proxy_api` -> `HARDCODED_PROXIES` 数组 -> 网页回退。

## 4. 字体反爬（核心技术点）

番茄正文把部分汉字替换为 Unicode 私用区（PUA，U+E000-U+F8FF）字符，用自定义字体渲染。直接抓 HTML 得到乱码。

### 4.1 证据链

1. 正文页引用的子集字体：`https://lf6-awef.bytetos.com/obj/awesome-font/c/dc027189e0ba4cd.woff2`
2. 子集字体元数据：
   - `nameID 0: This is a subset font of SourceHanSansSC-Normal. © 2014-2020`
   - `nameID 2: Normal`
   - `nameID 5: Version 2.002;hotconv 1.0.116;makeotfexe 2.5.65601`
3. 子集字体 cmap 全部是 PUA -> `gidXXXXX`，无真字 Unicode
4. 完整字体：`SourceHanSansCN-Normal.otf` v2.002，取自 Arch Archive 包 `adobe-source-han-sans-cn-fonts-2.002-1-any.pkg.tar.zst`

### 4.2 匹配方法

逐字形用 `RecordingPen` 取绘制指令，坐标四舍五入到 0.1，序列化后取 MD5。子集 PUA 字形的哈希在完整字体 cmap 字形中反查。

### 4.3 结果

- 362 / 362 全命中
- 跨书验证：`十日终焉`、`心肝宠`、`特工易冷` 三本不同书，字体哈希均为 `dc027189e0ba4cd`，PUA 码点全部落在映射表内
- 解码后 PUA 残留：0

### 4.4 后处理（7 个部首补充区字符 NFKC 不归一）

`⻋`->车、`⺠`->民、`⻢`->马、`⻔`->门、`⻅`->见、`⻛`->风、`⻓`->长

## 5. HTTP API

| 路径 | 参数 | 说明 |
|---|---|---|
| `GET /` | — | 端点列表 + 字体映射条目数 |
| `GET /api/search` | `key` | 搜索。网页路由实测 404，需代理；`key` 为 15 位以上数字时按 book_id 处理 |
| `GET /api/detail` | `book_id` 或 URL | 书籍详情 |
| `GET /api/directory` | `book_id` 或 URL | 章节目录（正序，`order` 从 1 开始） |
| `GET /api/content` | `item_id` 或 URL | 单章正文（已解码） |
| `GET /api/download` | `book_id`、`start`、`count` | 分段下载，`count ≤ 30`，返回 `next` |
| `GET/POST /api/config` | JSON `{"proxy_api":"..."}` | 读/写代理（POST 需 KV） |
| `POST /tg/webhook` | — | Telegram 回调 |
| `GET /tg/setup` | — | 注册 Telegram webhook |

`book_id` / `item_id` 均支持直接传番茄 URL，`parseBookId()` 自动提取数字 ID。以下 5 种形式均解析为 `7143038691944959011`（实测 7/7）：

~~~
7143038691944959011
https://fanqienovel.com/page/7143038691944959011
https://fanqienovel.com/page/7143038691944959011?enter_from=search
https://fanqienovel.com/reader/7173216089122439711
https://fanqienovel.com/page/7143038691944959011/
~~~

## 6. Telegram Bot

### 6.1 环境变量

| 变量 | 必需 | 说明 |
|---|---|---|
| `TG_BOT_TOKEN` | 是 | @BotFather 获取 |
| `TG_WEBHOOK_SECRET` | 否 | 设置后校验 `x-telegram-bot-api-secret-token` 头 |

### 6.2 部署后两步

1. Cloudflare 后台 Settings -> Variables 添加 `TG_BOT_TOKEN`（加密保存）
2. 浏览器访问 `https://<worker>/tg/setup`，返回 `{"ok":true,...}` 即注册成功

### 6.3 用法

~~~
你: /start
Bot: 发送番茄小说链接或 book_id 即可下载...

你: https://fanqienovel.com/page/7096667097870109733
Bot: 《心肝宠》共 987 章，已启动加速下载（约 22 轮 × 数秒），完成后自动发送全书 TXT。
Bot: 《心肝宠》进度 270/987 (27%)
Bot: 《心肝宠》进度 495/987 (50%)
Bot: 《心肝宠》进度 765/987 (77%)
Bot: [心肝宠.txt] 全书 987 章
Bot: 《心肝宠》全部 987 章已发送完毕
~~~

## 7. 并发方案对比

| 方案 | 987 章耗时 | 成本 | 配置 |
|---|---|---|---|
| Durable Objects | 约 22 秒 | 免费额度 | `[[durable_objects.bindings]]` |
| Cron + KV | 约 22 分钟 | 免费 | `[[kv_namespaces]]` + `[triggers]` |
| 单次降级 | 30 章 | 免费 | 无绑定 |

### 7.1 Durable Objects 链式 alarm

~~~javascript
// 首轮：100ms 后触发
await this.state.storage.setAlarm(Date.now() + 100);
// 每轮抓 45 章后：1s 后触发下一轮
await this.state.storage.setAlarm(Date.now() + 1000);
~~~

每轮 `BATCH = 45` 章，并发 15。单轮子请求 45 reader + 1 TG = 46 < 50（免费版上限）。DO 实例本身不占请求配额。

mock 实测：

~~~
初始 alarm 间隔 = -135 ms
round 1:  cursor=45   nextAlarm=+1000ms  tg=-
round 6:  cursor=270  nextAlarm=+1000ms  tg=sendMessage   <- 25%
round 11: cursor=495  nextAlarm=+1000ms  tg=sendMessage   <- 50%
round 17: cursor=765  nextAlarm=+1000ms  tg=sendMessage   <- 75%
round 22: DONE                            tg=sendDocument,sendMessage
总轮数: 22
~~~

### 7.2 Cron + KV 兜底

Cron 每分钟触发 `scheduled()`，从 KV 取队列头任务，抓 45 章写回。mock 实测 22 轮，每轮子请求 44-46。

## 8. 部署

~~~bash
npm install -g wrangler
wrangler login          # 或 export CLOUDFLARE_API_TOKEN=xxx
cd workers
npx wrangler deploy
~~~

预期：

~~~
Total Upload: 27.65 KiB / gzip: 8.06 KiB
env.FANQIE_DO (FanqieJob)  Durable Object
Deployed fanqie-downloader triggers:
  https://fanqie-downloader.<sub>.workers.dev
~~~

启用 TG：

~~~bash
curl "https://fanqie-downloader.<sub>.workers.dev/tg/setup"
~~~

## 9. 已知限制

| 项 | 限制 | 影响 |
|---|---|---|
| 搜索 | 网页 `/search` 实测 404 | 必须配代理或传 book_id |
| 目录标题 | `page.chapterList` 只含最新 50 章 | 早期章节 `title` 为空，正文不受影响 |
| 单请求子请求 | 免费版 50 | 已按 45 章/轮设计 |
| KV 单 value | 25 MB | 约 5000 章 × 3000 字接近上限 |
| KV 写配额 | 1000 写/天（免费） | 987 章约 66 写 |
| TG 单文件 | 45 MB（已截断保护） | 超限截断并标注 |

## 10. 为什么没有实现官方 API 签名

`libmetasec_ml.so` 的 `readelf` 结果：

~~~
NEEDED: liblog.so libdl.so libandroid.so libm.so libc.so
SONAME: libmetasec_ml.so
EXPORTS: JNI_OnLoad (0x2a5194, 820 bytes)  <- 唯一导出符号
~~~

- 不依赖 `libc++_shared.so`，C++ 运行时已静态链接
- `X-Gorgon` / `X-Argus` / `X-Ladon` 全部通过 `JNI_OnLoad` 动态注册，符号表无任何可导出入口
- 纯 JS/WASM 无法还原，需 Unidbg 模拟执行 ARM64 指令

因此本 Worker 不直连官方移动端 API，只走网页回退或代理。这与原仓库 `src/main.py` 的设计一致——它本身就把目录/正文交给 `FANQIE_PROXY_API`。

## 11. 本地验证记录

| 测试项 | 结果 |
|---|---|
| `parseBookId` 5 种 URL 形式 | 7/7 通过 |
| `parseState` + `decodeFont`（心肝宠 第 1 章） | 1215 字，PUA 残留 0 |
| `parseState` + `decodeFont`（特工易冷 第 1 章） | 4850 字，PUA 残留 0 |
| `hDetail`（心肝宠） | 九棂 / 987 章 / from: web |
| `hDirectory`（心肝宠） | 987 条，首尾 id 正确 |
| `hDownload start=0 count=2` | 2 章，标题正文正确，next=2 |
| TG `/start` | sendMessage |
| TG 发 URL | sendMessage + sendDocument + sendMessage |
| DO 链式 alarm（987 章） | 22 轮，每轮 +1000ms |
| Cron 队列（987 章） | 22 轮，每轮子请求 44-46 |
| `node --check worker.mjs` | SYNTAX_OK |
| `wrangler deploy --dry-run` | 27.65 KiB / gzip 8.06 KiB |

## 12. 运行时边界

所有逻辑都在 Cloudflare 平台内闭环，无需自建服务器：

| 组件 | 运行位置 |
|---|---|
| HTTP API | Workers |
| Telegram Bot | Workers + TG 服务器 |
| 网页抓取 | Workers `fetch()` |
| 字体解码 | Workers 内存查表 |
| 全本队列 | Durable Objects / KV |
| 定时触发 | Cron Triggers |

唯一的外部依赖：

1. **代理 API**（可选）：仅 `/api/search` 需要，详情/目录/正文/下载/TG 都不依赖
2. **字体映射表**：离线一次性算好后硬编码进 `worker.js`，运行时不做任何字形计算

## 13. 长域名短链支持

番茄分享链接有多种形式，`parseBookId()` 全部支持：

| 形式 | 示例 | 解析方式 |
|---|---|---|
| 纯 book_id | `7096667097870109733` | 直接匹配 |
| fanqie 页面 | `https://fanqienovel.com/page/7096667097870109733` | 正则 |
| fanqie 阅读页 | `https://fanqienovel.com/reader/7173216089122439711` | 正则 |
| 带查询参数 | `...page/7096667097870109733?enter_from=search` | 正则 |
| 长域名短链 | `https://changdunovel.com/t/oVotJ8xGHP4/` | 读 302 Location 里的 `book_id` |
| share-v2 | `https://changdunovel.com/wap/share-v2.html?book_id=xxx` | 查询参数 |
| book-share | `https://changdunovel.com/ug/pages/book-share?book_id=xxx` | 查询参数 |

短链实测：`changdunovel.com/t/oVotJ8xGHP4/` → 302 → `Location: ...book_share?book_id=7592279900040465433`，提取成功。本地 mock 8/8 用例全通过。

关键实现：用 `redirect: "manual"` 读 302 响应头，而不是 `follow` 后去匹配 `/page/` 路径——因为落点仍是 `book-share?book_id=`，不是 `/page/`。

## 14. 导出格式

### 14.1 `/api/export`（JSON 片段）

| format | 返回字段 | 用途 |
|---|---|---|
| `txt` | `text` | 纯文本片段，`\n\n` 分隔章节 |
| `html` | `html` | `<section>` 片段，带章节锚点 |
| `epub` | `chapters[]` | 每章 `{id, title, xhtml}` |

### 14.2 `/api/export/zip`（二进制 zip 直接下载）

| format | zip 内文件 |
|---|---|
| `txt` | `书名_1-N.txt`（单文件，带 BOM） |
| `html` | `index.html` + `chapter_00001_<id>.html` + ... |

响应头携带分页信息：`x-total`、`x-start`、`x-count`、`x-next`（空字符串表示已到末尾）。

ZIP 由自写 `makeZip()` 生成（STORE 模式无压缩），CRC32 多项式 `0xEDB88320`。验证：
- `crc32("123456789")` = `cbf43926`（标准测试向量）
- Python `zipfile.testzip()` 返回 `None`
- 中英文文件名与内容均正确解码

### 14.3 整本下载脚本

~~~bash
BASE="https://fanqie-downloader.3942369144a.workers.dev"
BOOK="7592279900040465433"
START=0
: > "book_${BOOK}.txt"
while :; do
  R=$(curl -sS "$BASE/api/export?book_id=$BOOK&format=txt&start=$START&count=40")
  echo "$R" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d.get("text",""))' >> "book_${BOOK}.txt"
  NEXT=$(echo "$R" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d.get("next") or "")')
  [ -z "$NEXT" ] && break
  START=$NEXT
done
wc -l "book_${BOOK}.txt"
~~~

## 15. 更新检查 `/api/update`

对比 KV 里记录的章节数，返回新增章节数：

~~~json
{
  "code": 200,
  "book_id": "7592279900040465433",
  "book_name": "我操作的少女正在成为旧日之主",
  "total": 503,
  "last_seen": 0,
  "new_chapters": 503,
  "has_kv": false,
  "next_start": 0
}
~~~

- `has_kv: false` 表示未绑定 KV，每次全量
- 绑定 KV 后 `last_seen` 记录上次章节数，`new_chapters` 只返回增量
- `next_start` 直接传给 `/api/export` 的 `start`，只拉新增章节

## 16. 与原 `main.py` 功能对照

| 原功能 | 原实现 | Worker 实现 | 状态 |
|---|---|---|---|
| TXT 下载 | `_download_txt` | `/api/export?format=txt` + `/api/export/zip` | 已实现 |
| HTML 下载 | `_download_html` | `/api/export?format=html` + `/api/export/zip` | 已实现 |
| EPUB 下载 | `_download_epub` | `/api/export?format=epub`（xhtml 片段） | 已实现 |
| 作者信息 | `_get_author_info` | `extractLdJson()` 抓 `author[0].name` | 线上验证 |
| 封面 URL | `_get_cover_url` | `extractLdJson()` 抓 `image[0]` | 线上验证 |
| 更新记录 | `_update_records` | `/api/update` + KV `record:<id>` | 已实现 |
| 搜索 | `search_novel` | `/api/search`（需代理） | 降级 |
| 批量下载 | `download_novel` 循环 | 客户端循环 `/api/export` | 已实现 |
| 文件名清洗 | `_sanitize_filename` | 正则替换非法字符为 `_` | 已实现 |
| 章节排序 | 按 `chapters.keys()` | `itemIds` reverse 后 `order` 递增 | 已实现 |

## 17. 从零部署教程

### 17.1 前置条件

- Cloudflare 账号（免费版即可）
- 一个 Telegram Bot（@BotFather 创建）
- 可选：GitHub 账号（Git 集成部署）

### 17.2 方式一：REST API 部署（无需 wrangler）

第 1 步，创建 API Token：Dashboard → My Profile → API Tokens → Create Token → 模板选 Edit Cloudflare Workers。

第 2 步，获取 Account ID：Dashboard 右侧栏，32 位十六进制。

第 3 步，准备 `metadata.json`：

~~~json
{
  "main_module": "worker.js",
  "compatibility_date": "2026-10-24",
  "compatibility_flags": ["nodejs_compat"],
  "bindings": [
    {"type": "durable_object_namespace", "name": "FANQIE_DO", "class_name": "FanqieJob"}
  ],
  "exports": {
    "FanqieJob": {"type": "durable-object", "storage": "sqlite"}
  }
}
~~~

首次部署用 `migrations`，之后更新必须改用 `exports`，否则报错 10074（Cannot apply new-sqlite-class migration to class that is already depended on by existing Durable Objects）。

第 4 步，上传脚本：

~~~bash
curl -X PUT "https://api.cloudflare.com/client/v4/accounts/<ACCOUNT_ID>/workers/scripts/fanqie-downloader" \
  -H "Authorization: Bearer <API_TOKEN>" \
  -H "Content-Type: multipart/form-data" \
  --form "metadata=@metadata.json;type=application/json" \
  --form "worker.js=@worker.js;type=application/javascript+module"
~~~

第 5 步，启用 workers.dev：

~~~bash
curl -X POST "https://api.cloudflare.com/client/v4/accounts/<ACCOUNT_ID>/workers/scripts/fanqie-downloader/subdomain" \
  -H "Authorization: Bearer <API_TOKEN>" \
  -H "Content-Type: application/json" \
  --data '{"enabled":true}'
~~~

第 6 步，设置 TG token：

~~~bash
curl -X PUT "https://api.cloudflare.com/client/v4/accounts/<ACCOUNT_ID>/workers/scripts/fanqie-downloader/secrets" \
  -H "Authorization: Bearer <API_TOKEN>" \
  -H "Content-Type: application/json" \
  --data '{"name":"TG_BOT_TOKEN","text":"<真实token>","type":"secret_text"}'
~~~

第 7 步，注册 Webhook：

~~~bash
curl "https://fanqie-downloader.<subdomain>.workers.dev/tg/setup"
~~~

返回 `{"ok":true,"telegram":{"ok":true,"description":"Webhook was set"}}` 即成功。

### 17.3 方式二：wrangler 部署

~~~bash
npm install -g wrangler
export CLOUDFLARE_API_TOKEN="<token>"
export CLOUDFLARE_ACCOUNT_ID="<account_id>"
cd workers
npx wrangler deploy
npx wrangler secret put TG_BOT_TOKEN
curl "https://fanqie-downloader.<sub>.workers.dev/tg/setup"
~~~

容器内无浏览器时 `wrangler login` 会因缺 `xdg-open` 失败，必须用 `CLOUDFLARE_API_TOKEN` 环境变量。若 wrangler 报 `fetch failed` 而 curl 正常，是代理环境干扰 undici，改用 17.2 的 REST API 方式。

### 17.4 方式三：Git 集成

1. Dashboard → Workers & Pages → Create → Connect to Git
2. 选仓库 `a8219/fanqiedownloader`，Root directory 填 `workers`
3. Settings → Variables 加 `TG_BOT_TOKEN`
4. Save and Deploy

之后 `git push` 到 `main` 自动部署。

## 18. Workers 平台限制与应对

| 限制 | 免费版 | Worker 的应对 |
|---|---|---|
| 单请求子请求数 | 50 | 每轮最多 45 章（45 reader + 1 TG） |
| 单请求 wall clock | 30 秒 | DO 每轮 1 秒内完成 |
| Durable Object | 100 万请求/月 | 987 章约 22 次 |
| KV 读 | 10 万/天 | 可选，仅 update 用 |
| KV 写 | 1000/天 | 987 章约 66 写 |
| Cron | 1440 次/天 | 每分钟一次够用 |
| TG 上传 | 50 MB | 已加 45 MB 截断保护 |

核心设计：把「抓完整本」拆成多次独立请求，每次控制在 50 子请求以内。

## 19. 实测记录（2026-10-24 线上）

| 测试 | 结果 |
|---|---|
| 根路由 | 11 个端点 + `font_map_entries: 362` |
| `/api/detail?book_id=<短链>` | `我操作的少女正在成为旧日之主 / 陷阵营营长 / 503 章` |
| `/api/detail` 作者+封面 | `九棂` + `https://p6-novel-sign.byteimg.com/...` |
| `/api/directory` | 503 条，正序 |
| `/api/content` | PUA 残留 0 |
| `/api/download count=2` | `total: 987, next: 2, PUA 残留 0` |
| `/api/export format=txt` | `len: 5645, next: 2` |
| `/api/export format=html` | `<section>` 片段，3410 字符 |
| `/api/export format=epub` | `chapters: 1`，xhtml 完整 |
| `/api/export/zip format=txt` | 16524 字节，Python zipfile 验证通过 |
| `/api/export/zip format=html` | 3 个文件（index + 2 章） |
| `/api/update` | `total: 503, last_seen: 0, has_kv: false` |
| `/api/search` | `501` + 明确提示（预期降级） |
| `/api/detail book_id=abc` | `400` 参数校验 |
| `/api/detail book_id=不存在` | `502` 上游 404 透传 |
| TG webhook | `Webhook was set`，`last_error: (无)` |
| DO 命名空间 | `FanqieJob / cd1fe7ef962a4592a4008de77ddfe4dc` |
