# Workers 部署说明

## 绑定

`wrangler.toml` 中需要两项绑定：

- `FANQIE_KV`：KV 命名空间，执行 `wrangler kv namespace create FANQIE_KV` 后把 id 填入
- `FANQIE_DO`：Durable Object，由 `[[migrations]]` 自动创建

## 环境变量

| 变量 | 必填 | 说明 |
|---|---|---|
| `TG_BOT_TOKEN` | 是 | Telegram Bot Token |
| `FANQIE_ADMIN_USERNAME` | 是 | 管理员账号 |
| `FANQIE_ADMIN_PASSWORD` | 是 | 管理员密码 |
| `FANQIE_ONLINE_READ` | 否 | 在线阅读开关，`1` 启用 / `0` 禁用，默认 `0` |
| `FANQIE_COOKIE` | 否 | 番茄 Cookie |
| `FANQIE_PROXY_API` | 否 | 代理 API 基址 |
| `FANQIE_USE_PROXY` | 否 | `1` 启用代理池 |
| `TG_WEBHOOK_SECRET` | 否 | Webhook 校验串 |

管理员账号密码仅从环境变量读取，代码中无硬编码，也不写入 KV。

## 命令