#!/usr/bin/env python3
"""番茄小说下载 Telegram Bot

用户发送小说页URL → 自动下载 → 每5s更新进度 → 完成后重排序+补缺章 → 发送文件 → 删除本地文件

附加功能：
- 账号系统：/login 两步登录（账号→密码）；/signup 用户自助注册（用户名+密码）
- 管理面板：/admin（InlineKeyboardButton 回调）
  - 用户记录列表、每人今日/总下载次数
  - ban / unban 用户
  - 设置全局每日下载上限
- 下载限额：-1 无上限 / 0 禁止下载 / n 每日 n 次（失败不计）；管理员始终无上限
- 每条消息先记录 id + username + 昵称
- 被 ban 用户发任何消息只回固定文案
"""
import os
import re
import sys
import time
import json
import asyncio
import logging
import threading
from pathlib import Path
from datetime import date, datetime, timezone, timedelta

sys.path.insert(0, '/opt/fanqie/repo/src')
os.chdir('/opt/fanqie/repo/src')

from telegram import Update, InlineKeyboardButton, InlineKeyboardMarkup
from telegram.ext import (
    Application, CommandHandler, MessageHandler, ContextTypes, filters,
    CallbackQueryHandler,
)
from telegram.constants import ParseMode

from main import NovelDownloader, Config

logging.basicConfig(level=logging.INFO, format='%(asctime)s %(message)s')
logger = logging.getLogger(__name__)

BOT_TOKEN = os.environ.get('FANQIE_BOT_TOKEN')
if not BOT_TOKEN:
    raise RuntimeError('缺少环境变量 FANQIE_BOT_TOKEN')
DOWNLOADS_DIR = '/opt/fanqie/downloads'
DATA_DIR = '/opt/fanqie/data'
USERS_FILE = Path(DATA_DIR) / 'users.json'
SETTINGS_FILE = Path(DATA_DIR) / 'settings.json'
Path(DOWNLOADS_DIR).mkdir(parents=True, exist_ok=True)
Path(DATA_DIR).mkdir(parents=True, exist_ok=True)

# 管理员账密（环境变量）
ADMIN_USERNAME = os.environ.get('FANQIE_ADMIN_USERNAME', '')
ADMIN_PASSWORD = os.environ.get('FANQIE_ADMIN_PASSWORD', '')

# 默认全局每日下载上限（普通用户）：-1 无上限 / 0 禁止下载 / n 每日 n 次
DEFAULT_DAILY_LIMIT = 5

# 北京时区
CST = timezone(timedelta(hours=8))

BANNED_REPLY = '⛔ 您已被管理员封禁，如有疑问请联系管理员。'

_lock = threading.Lock()

# 下载任务状态: chat_id -> {task, status_msg, progress, total, done}
_tasks: dict = {}

# 登录会话: chat_id -> {'step': 'username'|'password'|'reg_password', 'username': str, 'mode': 'login'|'signup'}
_login_sessions: dict = {}

# 已登录管理员的 chat_id 集合
_admin_sessions: set = set()

# 已登录的普通账号: chat_id -> username
_user_sessions: dict = {}


def _valid_username(name: str) -> bool:
    """用户名 3~32 位，字母/数字/下划线/横线，首字符为字母或数字"""
    return bool(re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_-]{2,31}', name))


def _password_issues(pw: str) -> str:
    """返回不符合安全要求的原因，空串表示通过"""
    if len(pw) < 8:
        return '密码至少 8 位'
    if not re.search(r'[A-Za-z]', pw):
        return '密码需包含字母'
    if not re.search(r'[0-9]', pw):
        return '密码需包含数字'
    return ''


def _hash_pw(pw: str) -> str:
    import hashlib
    return hashlib.sha256(pw.encode('utf-8')).hexdigest()


def _account_exists(users: dict, name: str) -> bool:
    name = name.lower()
    for u in users.values():
        if str(u.get('account', '')).lower() == name:
            return True
    return False


# 登录日志: {chat_id: [{'t': iso, 'username': str, 'ok': bool, 'reason': str}]}
LOGIN_LOG = Path(DATA_DIR) / 'login_log.json'
_login_alert_sent: dict = {}  # chat_id -> last alert ts


def _log_login(chat_id: int, username: str, ok: bool, reason: str = ''):
    with _lock:
        data = _load_json(LOGIN_LOG, {})
        key = str(chat_id)
        data.setdefault(key, []).append({
            't': datetime.now(CST).isoformat(timespec='seconds'),
            'username': username,
            'ok': ok,
            'reason': reason,
        })
        # 只保留每个 chat 最近 50 条
        data[key] = data[key][-50:]
        _save_json(LOGIN_LOG, data)


def _recent_fails(chat_id: int, window_sec: int = 600) -> int:
    """最近 window_sec 秒内该 chat 的失败次数"""
    with _lock:
        data = _load_json(LOGIN_LOG, {})
        entries = data.get(str(chat_id), [])
    now = datetime.now(CST)
    n = 0
    for e in entries[-20:]:
        try:
            t = datetime.fromisoformat(e.get('t', ''))
        except Exception:
            continue
        if (now - t).total_seconds() <= window_sec and not e.get('ok'):
            n += 1
    return n


async def _alert_admins(ctx, text: str):
    """向所有已登录管理员会话推送告警"""
    for chat_id in list(_admin_sessions):
        try:
            await ctx.bot.send_message(chat_id=chat_id, text=text, parse_mode=ParseMode.HTML)
        except Exception:
            pass


def _load_login_log() -> dict:
    return _load_json(LOGIN_LOG, {})


def _today_str() -> str:
    """北京日期字符串，用于按日归零"""
    return datetime.now(CST).strftime('%Y-%m-%d')


# ---------------------------------------------------------------- 数据存储

def _load_json(path: Path, default):
    try:
        if path.exists():
            with open(path, 'r', encoding='utf-8') as f:
                return json.load(f)
    except Exception as e:
        logger.warning('读取 %s 失败: %s', path, e)
    return default


def _save_json(path: Path, data):
    tmp = str(path) + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    Path(tmp).replace(path)


def load_users() -> dict:
    """{chat_id_str: {id, username, first_name, today, today_count, total, banned, first_seen, last_seen,
                     account, pwd_hash}}"""
    return _load_json(USERS_FILE, {})


def save_users(users: dict):
    _save_json(USERS_FILE, users)


def load_settings() -> dict:
    return _load_json(SETTINGS_FILE, {'daily_limit': DEFAULT_DAILY_LIMIT})


def save_settings(settings: dict):
    _save_json(SETTINGS_FILE, settings)


def record_user(update: Update) -> dict:
    """记录/更新用户信息（每条消息都调用）。返回该用户的记录。"""
    if not update.effective_user:
        return {}
    uid = update.effective_user.id
    key = str(uid)
    today = _today_str()
    with _lock:
        users = load_users()
        u = users.get(key)
        if u is None:
            u = {
                'id': uid,
                'username': update.effective_user.username or '',
                'first_name': update.effective_user.first_name or '',
                'today': today,
                'today_count': 0,
                'total': 0,
                'banned': False,
                'first_seen': today,
                'last_seen': today,
            }
        else:
            u['username'] = update.effective_user.username or u.get('username', '')
            u['first_name'] = update.effective_user.first_name or u.get('first_name', '')
            u['last_seen'] = today
            if u.get('today') != today:
                u['today'] = today
                u['today_count'] = 0
        users[key] = u
        save_users(users)
    return u


def is_banned(uid) -> bool:
    with _lock:
        return bool(load_users().get(str(uid), {}).get('banned', False))


def is_admin(uid) -> bool:
    return uid in _admin_sessions


def get_daily_limit() -> int:
    return int(load_settings().get('daily_limit', DEFAULT_DAILY_LIMIT))


def check_and_consume_quota(uid) -> tuple:
    """返回 (ok, msg)。成功扣减一次当日配额；失败不扣。

    上限语义（用户独立值优先于全局）：-1 无上限 / 0 禁止下载 / n 每日 n 次
    """
    with _lock:
        users = load_users()
        u = users.get(str(uid))
        if u is None:
            return False, '用户记录不存在，请先发送 /start'
        today = _today_str()
        if u.get('today') != today:
            u['today'] = today
            u['today_count'] = 0
        if is_admin(uid):
            u['today_count'] = u.get('today_count', 0) + 1
            u['total'] = u.get('total', 0) + 1
            users[str(uid)] = u
            save_users(users)
            return True, ''
        limit = u.get('daily_limit')
        if limit is None:
            limit = get_daily_limit()
        limit = int(limit)
        if limit == 0:
            return False, '⛔ 你的账号当前被禁止下载，请联系管理员。'
        if limit < 0:
            u['today_count'] = u.get('today_count', 0) + 1
            u['total'] = u.get('total', 0) + 1
            users[str(uid)] = u
            save_users(users)
            return True, ''
        used = u.get('today_count', 0)
        if used >= limit:
            return False, f'⚠️ 今日下载次数已达上限（{limit} 次/天）。\n管理员无此限制，明天 0 点（北京时间）自动重置。'
        u['today_count'] = used + 1
        u['total'] = u.get('total', 0) + 1
        users[str(uid)] = u
        save_users(users)
        return True, ''


def refund_quota(uid):
    """下载失败时退还当日配额"""
    with _lock:
        users = load_users()
        u = users.get(str(uid))
        if not u:
            return
        if u.get('today_count', 0) > 0:
            u['today_count'] = u['today_count'] - 1
        if u.get('total', 0) > 0:
            u['total'] = u['total'] - 1
        users[str(uid)] = u
        save_users(users)


# 代理API（番茄内容接口）
PROXY_API = os.environ.get('FANQIE_PROXY_API', '')
import requests as req


def make_downloader():
    cfg = Config()
    cfg.xc = 32  # 线程数拉满
    cfg.save_path = DOWNLOADS_DIR
    cfg.bookstore_dir = '/opt/fanqie/repo/src/data/bookstore'
    cfg.type = 'txt'
    cfg.thread_num = 32
    return NovelDownloader(cfg)


def parse_novel_id(text: str) -> int | None:
    """从URL或纯数字中提取小说ID"""
    text = text.strip()
    m = re.search(r'/(?:page/)?(\d{10,})(?:\?|$|/|&)', text)
    if m:
        return int(m.group(1))
    m = re.search(r'[?&]book_id=(\d{10,})', text)
    if m:
        return int(m.group(1))
    m = re.match(r'^(\d{10,})$', text)
    if m:
        return int(m.group(1))
    return None


def resolve_short_url(url: str) -> int | None:
    """跟随短链重定向，从最终URL中提取小说ID"""
    try:
        r = req.get(url, timeout=15, allow_redirects=True)
        return parse_novel_id(r.url)
    except Exception:
        return None


def sort_and_fix(downloader, book_json_path: str, novel_id: int):
    """Use the authoritative remote directory, fill every missing chapter, and save atomically."""
    with open(book_json_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    name, remote, _ = downloader._get_chapter_list(novel_id)
    if name == 'err' or not remote:
        raise RuntimeError('无法获取远程章节目录')
    remote_by_num = {}
    for title, cid in remote.items():
        m = re.match(r'第\s*(\d+)\s*章', str(title).strip())
        if m:
            remote_by_num[int(m.group(1))] = (title, str(cid))
    existing = {}
    for title, content in data.items():
        m = re.match(r'第\s*(\d+)\s*章', str(title).strip())
        if m and content:
            existing[int(m.group(1))] = (title, content)
    missing = [n for n in sorted(remote_by_num) if n not in existing]
    fixed = {}
    for n in missing:
        title, cid = remote_by_num[n]
        for attempt in range(4):
            try:
                content = downloader._download_chapter_content(cid)
                if content and len(content.strip()) > 20:
                    data[title] = content
                    fixed[n] = (title, content)
                    break
            except Exception as e:
                logger.warning('补章 %s 第%s次失败: %s', title, attempt + 1, e)
            time.sleep(1)
    tmp = book_json_path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False)
    Path(tmp).replace(book_json_path)
    still_missing = [n for n in sorted(remote_by_num)
                     if n not in {int(re.match(r'第\s*(\d+)\s*章', str(k)).group(1))
                                  for k, v in data.items()
                                  if v and re.match(r'第\s*(\d+)\s*章', str(k))}]
    return len(remote_by_num), still_missing, fixed


def build_sorted_txt(downloader, book_json_path: str, out_path: str):
    """从JSON生成章节号有序的TXT"""
    with open(book_json_path, 'r', encoding='UTF-8') as f:
        d = json.load(f)
    items = []
    for title, content in d.items():
        if title == '_metadata':
            continue
        m = re.match(r'第\s*(\d+)\s*章', title)
        if m and content:
            items.append((int(m.group(1)), title, content))
    items.sort(key=lambda x: x[0])
    name = d.get('_metadata', {}).get('name') or out_path.rsplit('/', 1)[-1].rsplit('.', 1)[0]
    with open(out_path, 'w', encoding='UTF-8') as f:
        f.write(f'{name}\n\n')
        for n, title, content in items:
            f.write(f'{title}\n\n{content}\n\n')
    return len(items)


WELCOME = """在白嫖的vps上让ai弄的
可能会随时出一些问题
用搜书神器搜不到的番茄小说可以试试用这个
不建议作为主力使用

📚 使用方式
1. 发送 /signup 注册一个账号（用户名 + 密码）
2. 直接发送小说的 URL 或书籍 ID
3. 下载完成后机器人会回传 TXT 文件

🎚 下载限制
每人每天可下载一定次数，0 点（北京时间）自动重置
管理员不受限制；上限为 0 时全站暂停下载

如遇异常或被封禁，请联系机器人管理员。

📜 本机器人基于开源项目（AGPL-3.0）修改，源码见
https://github.com/a8219/fanqiedownloader"""


# ---------------------------------------------------------------- 登录/管理

async def cmd_start(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    record_user(update)
    text = get_welcome()
    ann = get_announcement()
    if ann:
        text += f'\n\n📢 <b>公告</b>\n{ann}'
    await update.message.reply_text(text, parse_mode=ParseMode.HTML)


async def cmd_login(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    """登录命令：/login，两步对话输入账号密码"""
    record_user(update)
    chat_id = update.effective_chat.id
    if chat_id in _login_sessions:
        await update.message.reply_text('⏳ 请先完成上一次操作。')
        return
    _login_sessions[chat_id] = {'step': 'username', 'mode': 'login'}
    await update.message.reply_text('🔐 请输入账号：')
    return


async def cmd_signup(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    """注册命令：/signup，两步对话设置用户名和密码"""
    record_user(update)
    chat_id = update.effective_chat.id
    if chat_id in _login_sessions:
        await update.message.reply_text('⏳ 请先完成上一次操作。')
        return
    _login_sessions[chat_id] = {'step': 'username', 'mode': 'signup'}
    await update.message.reply_text('📝 注册：请输入用户名（3~32 位，字母/数字/下划线/横线）')
    return


async def cmd_admin(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    """管理面板（仅登录成功的管理员）"""
    record_user(update)
    chat_id = update.effective_chat.id
    if not is_admin(chat_id):
        # 非管理员静默忽略
        return
    await update.message.reply_text(
        '🛠 管理面板',
        reply_markup=admin_menu_keyboard(),
    )


def admin_menu_keyboard() -> InlineKeyboardMarkup:
    return InlineKeyboardMarkup([
        [InlineKeyboardButton('👥 用户记录', callback_data='admin_users'),
         InlineKeyboardButton('📊 下载统计', callback_data='admin_stats')],
        [InlineKeyboardButton('🚫 封禁列表', callback_data='admin_bans'),
         InlineKeyboardButton('🎚 每日上限', callback_data='admin_limit')],
        [InlineKeyboardButton('📝 欢迎语', callback_data='admin_welcome'),
         InlineKeyboardButton('📢 公告', callback_data='admin_announce')],
        [InlineKeyboardButton('🔐 登录日志', callback_data='admin_logins'),
         InlineKeyboardButton('❌ 关闭', callback_data='admin_close')],
    ])


def get_welcome() -> str:
    return load_settings().get('welcome') or WELCOME


def get_announcement() -> str:
    return str(load_settings().get('announcement') or '').strip()


def _user_display(u: dict) -> str:
    un = u.get('username') or ''
    fn = u.get('first_name') or ''
    name = f'@{un}' if un else (fn or '（无昵称）')
    return f'{name}'


async def admin_users_page(update: Update, ctx: ContextTypes.DEFAULT_TYPE, offset: int = 0):
    users = load_users()
    items = sorted(users.values(), key=lambda x: -int(x.get('id', 0)))
    page_size = 8
    total_pages = max(1, (len(items) + page_size - 1) // page_size)
    page = max(0, min(offset, total_pages - 1))
    slice_ = items[page * page_size:(page + 1) * page_size]
    lines = [f'👥 用户记录（第 {page + 1}/{total_pages} 页，共 {len(items)} 人）\n']
    for u in slice_:
        ban = '🚫' if u.get('banned') else '✅'
        lines.append(
            f'{ban} {_user_display(u)}\n'
            f'   ID: <code>{u.get("id")}</code>  今日: {u.get("today_count", 0)}  总计: {u.get("total", 0)}'
        )
    if not slice_:
        lines.append('暂无用户记录')
    text = '\n'.join(lines)
    buttons = []
    row = []
    if page > 0:
        row.append(InlineKeyboardButton('⬅️ 上一页', callback_data=f'admin_users:{page - 1}'))
    if page < total_pages - 1:
        row.append(InlineKeyboardButton('➡️ 下一页', callback_data=f'admin_users:{page + 1}'))
    if row:
        buttons.append(row)
    for u in slice_:
        uid = u.get('id')
        dl = u.get('daily_limit')
        limit_tag = ''
        if dl is not None:
            limit_tag = ' | ' + {0: '上限:禁止', -1: '上限:∞'}.get(int(dl), f'上限:{int(dl)}')
        if u.get('banned'):
            buttons.append([InlineKeyboardButton(f'✅ 解封 {_user_display(u)}', callback_data=f'admin_unban:{uid}')])
        else:
            buttons.append([InlineKeyboardButton(f'🚫 封禁 {_user_display(u)}', callback_data=f'admin_ban:{uid}')])
        buttons.append([InlineKeyboardButton(f'🎚 设上限 {_user_display(u)}{limit_tag}', callback_data=f'admin_setlimit:{uid}')])
    buttons.append([InlineKeyboardButton('🔙 返回', callback_data='admin_menu')])
    return text, InlineKeyboardMarkup(buttons)


async def admin_bans_page(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    users = load_users()
    banned = [u for u in users.values() if u.get('banned')]
    lines = [f'🚫 封禁列表（{len(banned)} 人）\n']
    for u in banned:
        lines.append(f'{_user_display(u)}  ID: <code>{u.get("id")}</code>')
    if not banned:
        lines.append('当前没有被封禁的用户')
    text = '\n'.join(lines)
    buttons = [
        [InlineKeyboardButton(f'✅ 解封 {_user_display(u)}', callback_data=f'admin_unban:{u.get("id")}')]
        for u in banned
    ]
    buttons.append([InlineKeyboardButton('🔙 返回', callback_data='admin_menu')])
    return text, InlineKeyboardMarkup(buttons)


async def admin_callback(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    """InlineKeyboardButton 回调总入口"""
    query = update.callback_query
    if not query:
        return
    uid = query.from_user.id if query.from_user else None
    if uid is None or not is_admin(uid):
        try:
            await query.answer('⛔ 无权限', show_alert=True)
        except Exception:
            pass
        return
    data = query.data or ''
    try:
        await query.answer()
    except Exception:
        pass

    # 任何面板跳转都先清掉等待输入状态，避免返回后仍吞掉用户消息
    # 除非本次回调本身就是要进入等待输入的
    if data not in ('admin_limit', 'admin_welcome', 'admin_announce', 'admin_setlimit'):
        for k in ('awaiting_limit', 'awaiting_welcome', 'awaiting_announce', 'awaiting_setlimit'):
            ctx.user_data.pop(k, None)

    if data == 'admin_menu':
        await query.message.edit_text('🛠 管理面板', reply_markup=admin_menu_keyboard())
        return

    if data == 'admin_users':
        text, kb = await admin_users_page(update, ctx, 0)
        await query.message.edit_text(text, reply_markup=kb, parse_mode=ParseMode.HTML)
        return

    if data.startswith('admin_users:'):
        offset = int(data.split(':', 1)[1])
        text, kb = await admin_users_page(update, ctx, offset)
        await query.message.edit_text(text, reply_markup=kb, parse_mode=ParseMode.HTML)
        return

    if data == 'admin_bans':
        text, kb = await admin_bans_page(update, ctx)
        await query.message.edit_text(text, reply_markup=kb, parse_mode=ParseMode.HTML)
        return

    if data.startswith('admin_ban:') or data.startswith('admin_unban:'):
        action, target = data.split(':', 1)
        with _lock:
            users = load_users()
            t = users.get(str(target))
            if t is None:
                try:
                    await query.message.reply_text('⚠️ 未找到该用户')
                except Exception:
                    pass
                return
            t['banned'] = action == 'admin_ban'
            users[str(target)] = t
            save_users(users)
        verb = '已封禁' if action == 'admin_ban' else '已解封'
        await query.message.reply_text(
            f'{"🚫" if action == "admin_ban" else "✅"} {verb}：{_user_display(t)}（<code>{target}</code>）',
            parse_mode=ParseMode.HTML,
        )
        # 刷新封禁列表
        if str(query.message.text or '').startswith('🚫'):
            text, kb = await admin_bans_page(update, ctx)
            try:
                await query.message.edit_text(text, reply_markup=kb, parse_mode=ParseMode.HTML)
            except Exception:
                pass
        return

    if data == 'admin_stats':
        users = load_users()
        today = _today_str()
        active_today = sum(1 for u in users.values() if u.get('today') == today and u.get('today_count', 0) > 0)
        banned_n = sum(1 for u in users.values() if u.get('banned'))
        total_dl = sum(int(u.get('total', 0)) for u in users.values())
        today_dl = sum(int(u.get('today_count', 0)) for u in users.values() if u.get('today') == today)
        await query.message.edit_text(
            '📊 下载统计\n\n'
            f'👤 总用户数：{len(users)}\n'
            f'🟢 今日活跃：{active_today}\n'
            f'🚫 已封禁：{banned_n}\n'
            f'📥 今日下载：{today_dl} 次\n'
            f'📚 历史下载：{total_dl} 次\n'
            f'🎚 每日上限：{get_daily_limit()} 次/人',
            reply_markup=InlineKeyboardMarkup(
                [[InlineKeyboardButton('🔙 返回', callback_data='admin_menu')]]
            ),
        )
        return

    if data == 'admin_limit':
        settings = load_settings()
        limit = settings.get('daily_limit', DEFAULT_DAILY_LIMIT)
        desc = {0: '⛔ 禁止下载（维护状态）', -1: '♾ 无上限'}.get(limit, f'每人 {limit} 次/天')
        await query.message.edit_text(
            f'🎚 全局每日下载上限\n\n'
            f'当前：{desc}\n'
            f'管理员始终不受限制。\n\n'
            f'发送新数值修改：\n'
            f'• 正整数（如 5）= 每人每天 5 次\n'
            f'• 0 = 禁止下载\n'
            f'• -1 = 无上限',
            parse_mode=ParseMode.HTML,
            reply_markup=InlineKeyboardMarkup(
                [[InlineKeyboardButton('🔙 返回', callback_data='admin_menu')]]
            ),
        )
        ctx.user_data['awaiting_limit'] = True
        return

    if data == 'admin_close':
        try:
            await query.message.delete()
        except Exception:
            pass
        return

    if data.startswith('admin_setlimit:'):
        target = data.split(':', 1)[1]
        with _lock:
            users = load_users()
            t = users.get(str(target))
        if t is None:
            await query.message.reply_text('⚠️ 未找到该用户')
            return
        cur = t.get('daily_limit')
        cur_desc = {0: '⛔ 禁止下载', -1: '♾ 无上限'}.get(
            int(cur) if cur is not None else None, f'{cur} 次/天') if cur is not None else '跟随全局'
        await query.message.edit_text(
            f'🎚 设置 {_user_display(t)} 的每日下载上限\n\n'
            f'当前：{cur_desc}\n'
            f'全局上限：{get_daily_limit()}\n\n'
            f'发送新数值：\n'
            f'• 正整数 = 该用户每天 n 次\n'
            f'• 0 = 禁止该用户下载\n'
            f'• -1 = 该用户无上限\n'
            f'• del = 恢复跟随全局上限\n'
            f'• /cancel = 取消',
            parse_mode=ParseMode.HTML,
            reply_markup=InlineKeyboardMarkup(
                [[InlineKeyboardButton('🔙 返回', callback_data='admin_users:0')]]
            ),
        )
        ctx.user_data['awaiting_setlimit'] = target
        return

    if data == 'admin_welcome':
        cur = get_welcome()
        await query.message.edit_text(
            f'📝 欢迎语设置\n\n'
            f'当前欢迎语：\n{cur}\n\n'
            f'直接发送新的欢迎语内容即可修改（发送 /cancel 取消）。'
            f'当前公告：{"（空）" if not get_announcement() else get_announcement()}',
            reply_markup=InlineKeyboardMarkup(
                [[InlineKeyboardButton('🔙 返回', callback_data='admin_menu')]]
            ),
        )
        ctx.user_data['awaiting_welcome'] = True
        return

    if data == 'admin_announce':
        cur = get_announcement()
        await query.message.edit_text(
            f'📢 公告设置\n\n'
            f'当前公告：\n{cur or "（空，无公告）"}\n\n'
            f'直接发送新公告内容即可修改；发送单个减号 - 清除公告；发送 /cancel 取消。'
            f'公告会附加在 /start 欢迎语下方展示给用户。',
            reply_markup=InlineKeyboardMarkup(
                [[InlineKeyboardButton('🔙 返回', callback_data='admin_menu')]]
            ),
        )
        ctx.user_data['awaiting_announce'] = True
        return

    if data == 'admin_logins':
        log = _load_login_log()
        lines = ['🔐 登录日志（按 chat 分组，各最近 8 条）\n']
        for chat, entries in log.items():
            acct = entries[-1].get('username', '?') if entries else '?'
            lines.append(f'Chat <code>{chat}</code>（最近账号：{acct}）')
            for e in entries[-8:]:
                mark = '✅' if e.get('ok') else '❌'
                reason = e.get('reason', '')
                lines.append(f'  {mark} {e.get("t", "")} {reason}')
            lines.append('')
        if not log:
            lines.append('暂无登录记录')
        await query.message.edit_text(
            '\n'.join(lines)[:3500],
            parse_mode=ParseMode.HTML,
            reply_markup=InlineKeyboardMarkup(
                [[InlineKeyboardButton('🔙 返回', callback_data='admin_menu')]]
            ),
        )
        return


async def download_task(app, chat_id: int, novel_id: int, url: str, status_msg):
    """后台下载任务（在线程池中运行）"""
    loop = asyncio.get_event_loop()
    state = {'progress': 0, 'total': 0, 'phase': 'init', 'done': False}

    def progress_cb(current, total, desc='', chapter_title=None):
        state['progress'] = current
        state['total'] = total
        state['phase'] = 'downloading'

    def log_cb(msg):
        logger.info(f'[{chat_id}] {msg}')

    def run():
        downloader = make_downloader()
        downloader.progress_callback = progress_cb
        downloader.log_callback = log_cb

        # 阶段1: 获取目录并设置断点文件路径（失败自动重试，最多 3 次）
        state['phase'] = 'downloading'
        state['progress'] = 0
        name0, chapters0, _ = downloader._get_chapter_list(novel_id)
        for attempt in range(3):
            if name0 != 'err' and chapters0:
                break
            state['retry'] = attempt + 1
            log_cb(f'获取目录失败（第 {attempt + 1}/3 次），5 秒后重试…')
            time.sleep(5)
            name0, chapters0, _ = downloader._get_chapter_list(novel_id)
        if name0 == 'err' or not chapters0:
            state['phase'] = 'error'
            state['error'] = '获取章节目录失败（已重试 3 次），请稍后再试'
            return None
        safe0 = downloader._sanitize_filename(name0)
        downloader.book_json_path = str(Path(downloader.bookstore_dir) / f'{safe0}.json')
        state['total'] = len(chapters0)
        out = downloader.download_novel(novel_id)
        if out == 'err':
            state['phase'] = 'error'
            state['error'] = '下载正文失败：没有成功获取章节内容'
            return None

        # download_novel 不写入 _metadata，先取书名再定位JSON
        name, chapters, status = downloader._get_chapter_list(novel_id)
        if name == 'err' or not name:
            state['phase'] = 'error'
            state['error'] = '获取书名失败'
            return None
        safe = downloader._sanitize_filename(name)
        candidates = [Path(downloader.bookstore_dir) / f'{safe}.json', Path(downloader.bookstore_dir) / f'{novel_id}_{safe}.json']
        book_json_path = next((p for p in candidates if p.exists()), candidates[0])

        # 阶段2: 排序+补缺
        state['phase'] = 'sorting'
        state['progress'] = 0
        state['total'] = 1
        try:
            total, still_missing, fixed = sort_and_fix(downloader, str(book_json_path), novel_id)
            state['total_chapters'] = total
            state['missing'] = still_missing
            state['fixed'] = fixed
        except Exception as e:
            logger.exception('sort_and_fix failed')
            state['missing'] = []
            state['fixed'] = {}

        # 阶段3: 生成TXT
        state['phase'] = 'building'
        out_path = Path(DOWNLOADS_DIR) / f'{safe}.txt'
        try:
            count = build_sorted_txt(downloader, str(book_json_path), str(out_path))
            if count <= 0:
                raise RuntimeError('生成结果为 0 章，已拒绝发送空文件')
            state['chapter_count'] = count
            state['out_path'] = str(out_path)
            state['done'] = True
            return state
        except Exception as e:
            logger.exception('build failed')
            state['phase'] = 'error'
            state['error'] = f'生成TXT失败: {e}'
            return None

    # 每5秒更新进度消息
    async def updater():
        while not state['done'] and state['phase'] != 'error':
            try:
                p = state['progress']
                t = state['total']
                phase = state['phase']
                if phase == 'downloading':
                    pct = (p / t * 100) if t else 0
                    text = f'📥 下载中… {p}/{t} ({pct:.0f}%)'
                elif phase == 'sorting':
                    fixed = state.get('fixed', {})
                    miss = state.get('missing') or []
                    if fixed:
                        text = f'🔧 补全缺章… 已补 {len(fixed)} 章'
                    elif miss:
                        text = f'⚠️ 检查缺失章节… 缺 {len(miss)} 章'
                    else:
                        text = '🔁 排序章节中…'
                elif phase == 'building':
                    text = '📄 生成TXT中…'
                elif phase == 'downloading' and state.get('retry'):
                    text = f'⏳ 目录获取失败，第 {state["retry"]}/3 次重试中…'
                else:
                    text = '⏳ 准备中…'
                try:
                    await status_msg.edit_text(text)
                except Exception as e:
                    if 'message is not modified' not in str(e).lower():
                        logger.debug('progress edit failed: %s', e)
            except Exception:
                pass
            await asyncio.sleep(5)

    updater_task = asyncio.ensure_future(updater())
    try:
        result = await loop.run_in_executor(None, run)
    finally:
        state['done'] = True
        updater_task.cancel()
        try:
            await updater_task
        except asyncio.CancelledError:
            pass
    return result


async def handle_message(update: Update, ctx: ContextTypes.DEFAULT_TYPE):
    chat_id = update.effective_chat.id
    text = update.message.text or ''

    # 1) 每条消息先记录用户
    user = record_user(update)

    # 2) 被 ban 用户只回固定文案
    if user.get('banned'):
        await update.message.reply_text(BANNED_REPLY)
        return

    # 3) 管理员正在等待输入新上限
    if ctx.user_data.get('awaiting_limit'):
        ctx.user_data['awaiting_limit'] = False
        m = re.match(r'^\s*(-?\d{1,2})\s*$', text)
        if m:
            n = int(m.group(1))
            if n == -1 or n == 0 or 1 <= n <= 99:
                with _lock:
                    settings = load_settings()
                    settings['daily_limit'] = n
                    save_settings(settings)
                desc = {0: '禁止下载', -1: '无上限'}.get(n, f'每人 {n} 次/天')
                await update.message.reply_text(f'✅ 每日下载上限已设为：{desc}')
                return
        await update.message.reply_text('⚠️ 无效数值，请输入 1~99 的整数、0（禁止）或 -1（无上限）。')
        return

    # 3.5) 管理员正在输入新欢迎语
    if ctx.user_data.get('awaiting_welcome'):
        ctx.user_data['awaiting_welcome'] = False
        if text.strip() == '/cancel':
            await update.message.reply_text('已取消修改。')
            return
        with _lock:
            settings = load_settings()
            settings['welcome'] = text
            save_settings(settings)
        await update.message.reply_text('✅ 欢迎语已更新。')
        return

    # 3.6) 管理员正在输入新公告
    if ctx.user_data.get('awaiting_announce'):
        ctx.user_data['awaiting_announce'] = False
        if text.strip() == '/cancel':
            await update.message.reply_text('已取消修改。')
            return
        if text.strip() == '-':
            with _lock:
                settings = load_settings()
                settings['announcement'] = ''
                save_settings(settings)
            await update.message.reply_text('✅ 公告已清除。')
            return
        with _lock:
            settings = load_settings()
            settings['announcement'] = text
            save_settings(settings)
        await update.message.reply_text('✅ 公告已发布，用户下次 /start 时会看到。')
        return

    # 3.7) 管理员正在为某个用户设置独立上限
    if ctx.user_data.get('awaiting_setlimit'):
        target = ctx.user_data.pop('awaiting_setlimit')
        val = text.strip()
        if val == '/cancel':
            await update.message.reply_text('已取消修改。')
            return
        with _lock:
            users = load_users()
            t = users.get(str(target))
            if t is None:
                await update.message.reply_text('⚠️ 未找到该用户。')
                return
            if val == 'del':
                t.pop('daily_limit', None)
                msg = f'✅ {_user_display(t)} 已恢复跟随全局上限。'
            else:
                m = re.match(r'^(-?\d{1,3})$', val)
                if not m:
                    await update.message.reply_text('⚠️ 无效数值，请输入整数、0、-1 或 del。')
                    # 保留等待状态让管理员重试
                    ctx.user_data['awaiting_setlimit'] = target
                    return
                n = int(m.group(1))
                if n == -1 or n == 0 or 1 <= n <= 999:
                    t['daily_limit'] = n
                    msg = f'✅ {_user_display(t)} 的每日上限已设为：' + {0: '禁止下载', -1: '无上限'}.get(n, f'{n} 次/天')
                else:
                    await update.message.reply_text('⚠️ 数值超出范围（1~999、0 或 -1）。')
                    ctx.user_data['awaiting_setlimit'] = target
                    return
            users[str(target)] = t
            save_users(users)
        await update.message.reply_text(msg)
        return

    # 4) 登录/注册会话：两步对话
    sess = _login_sessions.get(chat_id)
    if sess:
        mode = sess.get('mode', 'login')
        # ---- 第一步：账号 / 用户名
        if sess.get('step') == 'username':
            text_in = text.strip()
            if mode == 'signup':
                if not _valid_username(text_in):
                    await update.message.reply_text('⚠️ 用户名格式不正确（3~32 位，字母/数字/下划线/横线，首位为字母或数字）')
                    return
                with _lock:
                    users = load_users()
                    if _account_exists(users, text_in):
                        await update.message.reply_text('⚠️ 该用户名已被占用，请换一个。')
                        return
                sess['username'] = text_in
                sess['step'] = 'reg_password'
                await update.message.reply_text('🔑 请输入密码（至少 8 位，需同时包含字母和数字）')
                return
            sess['username'] = text_in
            sess['step'] = 'password'
            await update.message.reply_text('🔑 请输入密码：')
            return
        # ---- 第二步：密码（注册）
        if sess.get('step') == 'reg_password':
            new_user = sess.get('username', '')
            pw = text.strip()
            _login_sessions.pop(chat_id, None)
            issue = _password_issues(pw)
            if issue:
                await update.message.reply_text(f'⚠️ {issue}，请重新 /signup 注册。')
                return
            with _lock:
                users = load_users()
                if _account_exists(users, new_user):
                    await update.message.reply_text('⚠️ 该用户名已被占用，请换一个重新 /signup。')
                    return
                u = users.get(str(chat_id))
                if u is None:
                    # 新 chat：建立用户记录
                    u = {
                        'id': chat_id,
                        'username': update.effective_user.username or '',
                        'first_name': update.effective_user.first_name or '',
                        'today': _today_str(),
                        'today_count': 0,
                        'total': 0,
                        'banned': False,
                        'first_seen': _today_str(),
                        'last_seen': _today_str(),
                    }
                u['account'] = new_user
                u['pwd_hash'] = _hash_pw(pw)
                users[str(chat_id)] = u
                save_users(users)
            _log_login(chat_id, new_user, True, 'signup')
            await update.message.reply_text(f'✅ 注册成功！账号：{new_user}\n现在可以正常下载了。')
            return
        # ---- 第二步：密码（登录）
        if sess.get('step') == 'password':
            input_user = sess.get('username', '')
            input_pass = text.strip()
            _login_sessions.pop(chat_id, None)
            # 管理员凭据（环境变量）优先判定
            if ADMIN_USERNAME and input_user == ADMIN_USERNAME and ADMIN_PASSWORD and input_pass == ADMIN_PASSWORD:
                _admin_sessions.add(chat_id)
                _log_login(chat_id, input_user, True, 'admin')
                await update.message.reply_text('✅ 管理员登录成功！\n发送 /admin 打开管理面板。')
                return
            # 普通账号
            with _lock:
                users = load_users()
                u = _find_user_by_account(users, input_user)
                if u is not None and u.get('pwd_hash') == _hash_pw(input_pass):
                    _user_sessions[chat_id] = input_user
                    _log_login(chat_id, input_user, True, 'user')
                    await update.message.reply_text(f'✅ 登录成功！账号：{input_user}')
                    return
            # 失败：记录 + 频率告警
            _log_login(chat_id, input_user, False, 'bad_credentials')
            fails = _recent_fails(chat_id, 600)
            if fails >= 3:
                await update.message.reply_text('❌ 账号或密码错误。\n⚠️ 短时间内多次失败，已通知管理员。')
                await _alert_admins(ctx, f'⚠️ 异常登录告警\nChat ID: <code>{chat_id}</code>\n'
                                         f'用户名尝试：{input_user}\n'
                                         f'10 分钟内失败 {fails} 次',
                                    )
            else:
                await update.message.reply_text('❌ 账号或密码错误。')
            return

    novel_id = parse_novel_id(text)
    if not novel_id and text.startswith('http'):
        # 短链场景：跟随重定向后再解析
        msg = await update.message.reply_text('🔗 正在解析分享链接…')
        novel_id = resolve_short_url(text)
        try:
            await msg.delete()
        except Exception:
            pass
    if not novel_id:
        await update.message.reply_text(
            '⚠️ 无法识别小说URL或ID。\n\n'
            '请发送类似：https://fanqienovel.com/page/7615694942362422297'
        )
        return

    # 4.5) 账号门槛：未注册账号的用户不能下载（管理员除外）
    if not is_admin(chat_id):
        with _lock:
            users = load_users()
            u = users.get(str(chat_id))
            has_account = bool(u and u.get('account'))
        if not has_account:
            await update.message.reply_text(
                '🔒 需要先注册账号才能下载。\n\n'
                '发送 /signup 注册一个账号（用户名 + 密码），\n'
                '注册成功后即可下载。'
            )
            return

    # 5) 下载配额检查（先扣，失败退还）
    ok, quota_msg = check_and_consume_quota(chat_id)
    if not ok:
        await update.message.reply_text(quota_msg)
        return
    if chat_id in _tasks and not _tasks[chat_id].get('finished'):
        refund_quota(chat_id)
        await update.message.reply_text('⏳ 你有一个下载任务正在进行，请等待完成。')
        return

    status_msg = await update.message.reply_text('⏳ 正在获取小说信息…')
    _tasks[chat_id] = {'finished': False}

    app = ctx.application
    result = await download_task(app, chat_id, novel_id, text, status_msg)

    _tasks[chat_id]['finished'] = True

    if not result or result.get('phase') == 'error':
        # 失败不计次数，退还配额
        refund_quota(chat_id)
        err = (result or {}).get('error', '下载失败，请重试')
        try:
            await status_msg.edit_text(f'❌ {err}')
        except Exception:
            await update.message.reply_text(f'❌ {err}')
        return

    out_path = result['out_path']
    count = result.get('chapter_count', 0)
    miss = result.get('missing') or []
    fixed = result.get('fixed') or {}
    fname = Path(out_path).name

    # 编辑最终状态
    try:
        summary = f'✅ 下载完成：{count} 章'
        if fixed:
            summary += f'\n🔧 已补全缺章 {len(fixed)} 章'
        if miss:
            summary += f'\n⚠️ 仍有 {len(miss)} 章获取失败'
        await status_msg.edit_text(summary)
    except Exception:
        pass

    # 发送文件
    sent = False
    try:
        with open(out_path, 'rb') as f:
            await ctx.bot.send_document(
                chat_id=chat_id,
                document=f,
                filename=fname,
                caption=f'📕 {fname}\n共 {count} 章'
            )
        sent = True
    except Exception as e:
        logger.exception('send_document failed')
        refund_quota(chat_id)
        await update.message.reply_text(f'❌ 发送文件失败: {e}\n文件路径: {out_path}')

    # 发送成功后删除本地文件
    if sent:
        try:
            os.remove(out_path)
            logger.info(f'已删除本地文件: {out_path}')
        except Exception as e:
            logger.warning(f'删除本地文件失败: {out_path}: {e}')
        # 同时清理bookstore JSON
        try:
            book_json = Path('/opt/fanqie/repo/src/data/bookstore') / fname.replace('.txt', '.json')
            if book_json.exists():
                os.remove(book_json)
                logger.info(f'已删除JSON: {book_json}')
        except Exception as e:
            logger.warning(f'删除JSON失败: {e}')


def main():
    app = Application.builder().token(BOT_TOKEN).build()
    app.add_handler(CommandHandler('start', cmd_start))
    app.add_handler(CommandHandler('login', cmd_login))
    app.add_handler(CommandHandler('signup', cmd_signup))
    app.add_handler(CommandHandler('admin', cmd_admin))
    app.add_handler(CallbackQueryHandler(admin_callback, pattern=r'^admin_'))
    app.add_handler(MessageHandler(filters.TEXT, handle_message))
    logger.info('Bot starting...')
    app.run_polling(allowed_updates=Update.ALL_TYPES)


if __name__ == '__main__':
    main()
