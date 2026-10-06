# Fanqienovel-downloader（衍生修改版）

> 本仓库是 [ying-ck/fanqienovel-downloader](https://github.com/ying-ck/fanqienovel-downloader)（AGPL-3.0）的衍生修改版，由 [Astra](https://github.com/a8219) 维护。
> 感谢上游作者 qxqycb 提供的原始项目，本仓库的所有原始代码、文档与致谢均归属于上游项目。

## 本仓库相对上游的改动

在上游基础上，本衍生版增加了以下功能（**这些功能不会影响上游已有的本地命令行使用方式**）：

- **Telegram 机器人**（`tgbot.py`）：用户私聊发送番茄小说页面 URL 或书籍 ID，机器人自动下载并回传排序后的 TXT 文件
  - 为防止机器人滥用，tg下载有账号机制
  - 下载进度实时编辑同一条消息
  - 以远程完整章节目录为准补齐缺章，按章节号排序后输出
  - 支持短链自动解析
  - **账号系统**：管理员登录后可设置每位普通用户每日下载上限、查看用户下载记录、封禁（ban）异常用户
  - 管理员无下载上限
- **Web 服务器改造**（`src/server.py`）：`/api/download/<id>` 改为立即入队并返回 `202`，下载在后台异步进行；适合部署在低配 VPS 上
- **正文接口适配**：`src/main.py` 的目录/正文获取改为通过可配置的代理 API（环境变量 `FANQIE_PROXY_API`），减少对官方接口的直连压力
- 修复了若干在低配服务器上触发的问题（并发下载的进度文件原子写入、代理模式下的 Cookie 初始化等）

> 原上游的本地命令行版本（`src/main.py`、`src/ref_main.py`）与 Docker 部署方式**保持不变**，使用方法见下文。

## 衍生工具
1.[c.exe](https://github.com/ying-ck/fanqienovel-downloader/releases/tag/v1.1.13)用于检测番茄小说网页结构变化

2.[s.exe](https://github.com/qxqycb/search-novel)用于小说内容搜索，可搭配番茄小说下载器使用

3.[f.exe](https://github.com/qxqycb/novel-spilt)以文件大小来分割小说文件，可搭配番茄小说下载器使用

## 使用方法

### 本地程序

### v1.1.8版本及以上

1. 输入小说目录页面完整链接或者id下载
1. 输入id或链接直接下载
2. 输入1以更新，读取 `record.json` 中的id进行更新
3. 输入2进行搜索
4. 输入3进行批量下载
5. 输入4进入设置，可调整正文段首占位符，调整延时，小说存储位置，保存模式
6. 输入5进行备份下载的小说以及下载格式、段首空格等
7. 输入6退出程序



### 请注意！修改了设置中的每一个选项都会覆盖原来的数据，请仔细查看后在做出选择。若想修复默认选项，请将`config.json`文件删除


## Q&A
### Q1：
报错：` The above exception was the direct cause of the following exception:
urllib3.exceptions.ProxyError: ('Unable to connect to proxy', FileNotFoundError(2, 'No such file or directory')) The above exception was the direct cause of the following exception:
Traceback (most recent call last):
File "requests\adapters. py", line 667, in send
File "urllib3\connectionpool. py", line 843, in urlopen File "urllib3\util\retry. py", line 519, in increment
urllib3. exceptions. MaxRetryError: HTTPSConnectionPool(host='fanqienovel. com', port=443): Max retries exceeded with url: /page/7143038691944959011 (Caused by ProxyError('Unable to connect to proxy', FileNotFoundError(2, 'No such file or dire ctory'))) `
......
### A1：
网络错误，请检查网络连接(如：关闭代理、加速)

### Web 版

<img src="https://github.com/user-attachments/assets/2dfb008b-bdd7-4ff8-a020-cd1e5ede1dc9" width="500">
<img src="https://github.com/user-attachments/assets/8edee2b2-91e4-483b-bb9b-79d8b18e4a00" width="500">
<img src="https://github.com/user-attachments/assets/f4257f33-e25e-477c-8f51-6ce5949d881f" width="500">
<img src="https://github.com/user-attachments/assets/152638c2-43c1-49b6-a097-b50f1ac495e3" width="500">


Web版实现的功能
- 网页服务器下载完直接让你下载小说文件到本地，所以能远程放在容器或虚拟机中运行
- 有进度条，漂亮！
- 能透过 id 下载小说，也能用名字搜索小说，更能更新之前下载的小说
- 简洁的 UI 界面
- 队列设计，可以把好几本书加入队列，批量下载
- 在线阅读

你有3种方式运行 web 版。

1.直接执行exe文件

2. Python 运行

用 Git 克隆这个项目或直接下载项目的zip并解压。进入项目文件夹，新建虚拟环境，并用 `pip install -r requirements.txt` 来安装这个项目的 python 依赖。

接着进入`src`目录，用python 运行 `server.py`，并根据指示用浏览器开启 `http://localhost:12930`。
(注意：`python`版本3.8及以下版本下载项目`zip`或`git`时，`src`目录中,将原来的`main.py`删除，再把`main2.py`名称改为`main.py`)

3. Docker 运行

用 Git 克隆这个项目或直接下载项目的zip并解压。进入项目文件夹。

直接使用 `docker compose up` (或是 `docker compose up -d` 在后台运行) 构建并启动镜像。启动后用浏览器访问 `http://localhost:12930`。

下载的小说和个人数据 (`data` 文件夹) 会存在docker 卷里面，分别叫做 `fanqie_data` 和 `fanqie_downloads`。如果你想修改成某个特定的目录，可以修改 `docker-compose.yaml` 文件中的持久化用户数据部分。

### 手机版
#### 现在有一种方式可在手机上使用(只是ref_main.py,不是web版)

安装termux

换源：
`sed -i 's@^(.*deb.*stable main)$@#\1\ndeb https://mirrors.tuna.tsinghua.edu.cn/termux/apt/termux-main stable main@' $PREFIX/etc/apt/sources.list`
`apt update && apt upgrade`

`pip config set global.index-url https://pypi.tuna.tsinghua.edu.cn/simple`

安装包：
`pip install requests ebooklib tqdm beautifulsoup4`(注意：在ref_main.py中删掉tkinter的导入)

运行：
python ref_main.py

注意：运行环境配置正确，不要对应错误
安装lxml库可能报错，按照以下步骤解决：
CFLAGS="-O0" pip install lxml



## 集思广益

若各位使用者有什么意见或程序有什么错误，欢迎在lssues中讨论

## 免责声明

此程序旨在用于与Python网络爬虫和网页处理技术相关的教育和研究目的。不应将其用于任何非法活动或侵犯他人权利的行为。用户对使用此程序引发的任何法律责任和风险负有责任，作者和项目贡献者不对因使用程序而导致的任何损失或损害承担责任

在使用此程序之前，请确保遵守相关法律法规以及网站的使用政策，并在有任何疑问或担忧时咨询法律顾问

This program is designed for educational and research purposes related to Python web crawlers and web page processing technologies. It should not be used for any illegal activities or acts that violate the rights of others. Users are responsible for any legal liabilities and risks arising from the use of this program. The author and project contributors are not responsible for any losses or damages resulting from the use of the program.

Before using this program, please ensure compliance with relevant laws and regulations and the website's usage policies. Consult a legal advisor if you have any questions or concerns.

## 开源

本程序遵循[AGPL-3.0](https://github.com/ying-ck/fanqienovel-downloader?tab=AGPL-3.0-1-ov-file)开源。使用本程序源码时请注明来源，并同样使用此协议。


## Star趋势

![Stars](https://star-history.dera.page/svg?repos=ying-ck/fanqienovel-downloader&type=Date)

## 赞助

![afdian-qxqycb](https://github.com/user-attachments/assets/19495126-7f23-410f-9796-c000df3c4185)
爱发电：https://afdian.com/a/qxqycb
注意：赞助了不等于一定会有可以用的程序，请自行斟酌


## Telegram Bot 部署说明

### 环境变量

| 变量 | 必填 | 说明 |
|---|---|---|
| `FANQIE_BOT_TOKEN` | ✅ | Telegram Bot Token（从 [@BotFather](https://t.me/BotFather) 获取） |
| `FANQIE_PROXY_API` | ✅ | 代理 API 基址，用于获取搜索/目录/正文。**请使用你自己拥有或已获授权的 API** |
| `FANQIE_WEB_SECRET_KEY` | ❌ | Web 服务器的 session 密钥，不设则每次启动随机生成 |

管理员账号密码通过环境变量注入，**不要写进代码**：
- `FANQIE_ADMIN_USERNAME`
- `FANQIE_ADMIN_PASSWORD`

可用 systemd 的 `EnvironmentFile=/etc/fanqie/fanqie.env`（权限 0600，owner root）管理。

### 机器人用法

- 发送书籍 URL 或 ID → 自动下载 → 回传 TXT
- `/start` → 欢迎信息
- `/login` → 管理员登录/普通登录（触发后依次询问账号、密码）
- `/admin` → **仅对已登录的管理员响应**，弹出 inline 按钮管理面板；非管理员使用该命令**不会有任何回应**（与不存在的命令行为一致）

管理面板功能（底部按钮）：
- 查看所有使用过的用户（id、username、昵称）
- 查看每个用户的今日下载次数与累计下载次数
- 封禁 / 解封用户
- 设置全局每日下载上限（失败的重试不计入）

被封禁的用户发送任何消息只会收到固定的提示文案。

### 关于代理 API

本仓库**不提供**代理 API 服务，也不内置任何地址。`FANQIE_PROXY_API` 需要你自己配置一个兼容的接口：
- `GET /api/search?q=关键词`
- `GET /api/detail?id=书籍ID`
- `GET /api/directory?id=书籍ID`
- `GET /api/content?id=章节ID`

请使用你自建或已获授权的 API。**不要**将你不确定来源的第三方 API 地址公开到代码、Issue 或文档中。

## 开源协议

本仓库遵循 [AGPL-3.0](https://github.com/ying-ck/fanqienovel-downloader?tab=AGPL-3.0-1-ov-file)。作为上游的衍生作品，本仓库同样使用 AGPL-3.0 开源，使用本仓库源码时请注明来源（上游 ying-ck/fanqienovel-downloader 与本衍生仓库），并同样使用此协议。

