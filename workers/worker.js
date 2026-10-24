// 番茄小说下载器 — Cloudflare Workers 单文件版
// 依据: a8219/fanqiedownloader src/main.py 的代理 API 契约
//   /search?key=&tab_type=3 -> data.search_tabs[tab_type=3].data[].book_data[0]
//   /detail?book_id=        -> data.data.book_name / status
//   /directory?book_id=     -> data.lists[].title / item_id
//   /content?tab=小说&item_id= -> data.content
// 网页回退依据(实测 2026-10): /page/{book_id} 内 window.__INITIAL_STATE__.page
//   /reader/{item_id} 内 reader.chapterData.content, PUA 字体需 FONT_MAP 解码
// 字体映射来源: 子集 woff2 dc027189e0ba4cd 与 SourceHanSansCN-Normal v2.002
//   逐字形轮廓 MD5 匹配, 362/362 命中, 三本不同书验证 PUA 全覆盖

const FONT_MAP = {58344:"D",58345:"在",58346:"主",58347:"特",58348:"家",58349:"军",58350:"然",58351:"表",58352:"场",58353:"4",58354:"要",58355:"只",58356:"v",58357:"和",58359:"6",58360:"别",58361:"还",58362:"g",58363:"现",58364:"儿",58365:"岁",58368:"此",58369:"象",58370:"月",58371:"3",58372:"出",58373:"战",58374:"工",58375:"相",58376:"o",58377:"男",58378:"直",58379:"失",58380:"世",58381:"F",58382:"都",58383:"平",58384:"文",58385:"什",58386:"V",58387:"O",58388:"将",58389:"真",58390:"T",58391:"那",58392:"当",58394:"会",58395:"立",58396:"些",58397:"u",58398:"是",58399:"十",58400:"张",58401:"学",58402:"气",58403:"大",58404:"爱",58405:"两",58406:"命",58407:"全",58408:"后",58409:"东",58410:"性",58411:"通",58412:"被",58413:"1",58414:"它",58415:"乐",58416:"接",58417:"而",58418:"感",58419:"车",58420:"山",58421:"公",58422:"了",58423:"常",58424:"以",58425:"何",58426:"可",58427:"话",58428:"先",58429:"p",58430:"i",58431:"叫",58432:"轻",58433:"M",58434:"士",58435:"w",58436:"着",58437:"变",58438:"尔",58439:"快",58440:"l",58441:"个",58442:"说",58443:"少",58444:"色",58445:"里",58446:"安",58447:"花",58448:"远",58449:"7",58450:"难",58451:"师",58452:"放",58453:"t",58454:"报",58455:"认",58456:"面",58457:"道",58458:"S",58460:"克",58461:"地",58462:"度",58463:"I",58464:"好",58465:"机",58466:"U",58467:"民",58468:"写",58469:"把",58470:"万",58471:"同",58472:"水",58473:"新",58474:"没",58475:"书",58476:"电",58477:"吃",58478:"像",58479:"斯",58480:"5",58481:"为",58482:"y",58483:"白",58484:"几",58485:"日",58486:"教",58487:"看",58488:"但",58489:"第",58490:"加",58491:"候",58492:"作",58493:"上",58494:"拉",58495:"住",58496:"有",58497:"法",58498:"r",58499:"事",58500:"应",58501:"位",58502:"利",58503:"你",58504:"声",58505:"身",58506:"国",58507:"问",58508:"马",58509:"女",58510:"他",58511:"Y",58512:"比",58513:"父",58514:"x",58515:"A",58516:"H",58517:"N",58518:"s",58519:"X",58520:"边",58521:"美",58522:"对",58523:"所",58524:"金",58525:"活",58526:"回",58527:"意",58528:"到",58529:"z",58530:"从",58531:"j",58532:"知",58533:"又",58534:"内",58535:"因",58536:"点",58537:"Q",58538:"三",58539:"定",58540:"8",58541:"R",58542:"b",58543:"正",58544:"或",58545:"夫",58546:"向",58547:"德",58548:"听",58549:"更",58551:"得",58552:"告",58553:"并",58554:"本",58555:"q",58556:"过",58557:"记",58558:"L",58559:"让",58560:"打",58561:"f",58562:"人",58563:"就",58564:"者",58565:"去",58566:"原",58567:"满",58568:"体",58569:"做",58570:"经",58571:"K",58572:"走",58573:"如",58574:"孩",58575:"c",58576:"G",58577:"给",58578:"使",58579:"物",58581:"最",58582:"笑",58583:"部",58585:"员",58586:"等",58587:"受",58588:"k",58589:"行",58590:"一",58591:"条",58592:"果",58593:"动",58594:"光",58595:"门",58596:"头",58597:"见",58598:"往",58599:"自",58600:"解",58601:"成",58602:"处",58603:"天",58604:"能",58605:"于",58606:"名",58607:"其",58608:"发",58609:"总",58610:"母",58611:"的",58612:"死",58613:"手",58614:"入",58615:"路",58616:"进",58617:"心",58618:"来",58619:"h",58620:"时",58621:"力",58622:"多",58623:"开",58624:"已",58625:"许",58626:"d",58627:"至",58628:"由",58629:"很",58630:"界",58631:"n",58632:"小",58633:"与",58634:"Z",58635:"想",58636:"代",58637:"么",58638:"分",58639:"生",58640:"口",58641:"再",58642:"妈",58643:"望",58644:"次",58645:"西",58646:"风",58647:"种",58648:"带",58649:"J",58651:"实",58652:"情",58653:"才",58654:"这",58656:"E",58657:"我",58658:"神",58659:"格",58660:"长",58661:"觉",58662:"间",58663:"年",58664:"眼",58665:"无",58666:"不",58667:"亲",58668:"关",58669:"结",58670:"0",58671:"友",58672:"信",58673:"下",58674:"却",58675:"重",58676:"己",58677:"老",58678:"2",58679:"音",58680:"字",58681:"m",58682:"呢",58683:"明",58684:"之",58685:"前",58686:"高",58687:"P",58688:"B",58689:"目",58690:"太",58691:"e",58692:"9",58693:"起",58694:"稜",58695:"她",58696:"也",58697:"W",58698:"用",58699:"方",58700:"子",58701:"英",58702:"每",58703:"理",58704:"便",58705:"四",58706:"数",58707:"期",58708:"中",58709:"C",58710:"外",58711:"样",58712:"a",58713:"海",58714:"们",58715:"任"};

const HARDCODED_PROXIES = [
  // { name: "my", base: "https://proxy.example.com/api" }
];

const WEB_BASE = "https://fanqienovel.com";
const UA = "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36";
const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET,POST,OPTIONS",
  "access-control-allow-headers": "content-type",
};
const json = (o, s = 200) =>
  new Response(JSON.stringify(o), { status: s, headers: { "content-type": "application/json; charset=utf-8", ...CORS } });

async function fetchT(url, opts = {}, ms = 15000) {
  const c = new AbortController();
  const t = setTimeout(() => c.abort(), ms);
  try { return await fetch(url, { ...opts, signal: c.signal }); }
  finally { clearTimeout(t); }
}

async function parseBookId(v) {
  if (!v) return "";
  let s = String(v).trim();
  if (/^\d{10,}$/.test(s)) return s;

  // changdunovel.com/t/xxx/ 短链接：读 302 Location 里的 book_id
  if (/changdunovel\.com\/t\//.test(s)) {
    try {
      const r = await fetch(s, { redirect: "manual", headers: { "user-agent": UA } });
      const loc = r.headers.get("location") || "";
      const m1 = loc.match(/[?&]book_id=(\d{10,})/);
      if (m1) return m1[1];
      const m2 = loc.match(/\/(?:page|reader)\/(\d{10,})/);
      if (m2) return m2[1];
      // 退路：follow 一次拿落点
      const r2 = await fetch(s, { redirect: "follow", headers: { "user-agent": UA } });
      const m3 = r2.url.match(/[?&]book_id=(\d{10,})/);
      if (m3) return m3[1];
    } catch (e) {}
    return "";
  }

  // share-v2.html?book_id=xxx
  if (/changdunovel\.com\/wap\/share-v2\.html/.test(s)) {
    try {
      const bid = new URL(s).searchParams.get("book_id");
      if (bid && /^\d{10,}$/.test(bid)) return bid;
    } catch (e) {}
  }

  // book-share?book_id=xxx
  if (/changdunovel\.com\/ug\/pages\/book-share/.test(s)) {
    try {
      const bid = new URL(s).searchParams.get("book_id");
      if (bid && /^\d{10,}$/.test(bid)) return bid;
    } catch (e) {}
  }

  // fanqienovel.com/page/xxx 或 /reader/xxx
  const m = s.match(/\/(?:page|reader)\/(\d{10,})/);
  if (m) return m[1];

  return "";
}


// ---------- 最小 ZIP 生成器（store，无压缩） ----------
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(bytes) {
  let c = 0xFFFFFFFF;
  for (let i = 0; i < bytes.length; i++) c = CRC_TABLE[(c ^ bytes[i]) & 0xFF] ^ (c >>> 8);
  return (c ^ 0xFFFFFFFF) >>> 0;
}

function makeZip(files) {
  const enc = new TextEncoder();
  const locals = [];
  const centrals = [];
  let offset = 0;
  const now = new Date();
  const dosTime = ((now.getHours() << 11) | (now.getMinutes() << 5) | (now.getSeconds() >> 1)) & 0xFFFF;
  const dosDate = (((now.getFullYear() - 1980) << 9) | ((now.getMonth() + 1) << 5) | now.getDate()) & 0xFFFF;

  for (const f of files) {
    const nameBytes = enc.encode(f.name);
    const data = typeof f.data === "string" ? enc.encode(f.data) : f.data;
    const crc = crc32(data);
    const size = data.length;

    const lh = new Uint8Array(30 + nameBytes.length);
    const lv = new DataView(lh.buffer);
    lv.setUint32(0, 0x04034b50, true);
    lv.setUint16(4, 20, true);
    lv.setUint16(6, 0x0800, true);
    lv.setUint16(8, 0, true);
    lv.setUint16(10, dosTime, true);
    lv.setUint16(12, dosDate, true);
    lv.setUint32(14, crc, true);
    lv.setUint32(18, size, true);
    lv.setUint32(22, size, true);
    lv.setUint16(26, nameBytes.length, true);
    lv.setUint16(28, 0, true);
    lh.set(nameBytes, 30);

    locals.push(lh, data);

    const ch = new Uint8Array(46 + nameBytes.length);
    const cv = new DataView(ch.buffer);
    cv.setUint32(0, 0x02014b50, true);
    cv.setUint16(4, 20, true);
    cv.setUint16(6, 20, true);
    cv.setUint16(8, 0x0800, true);
    cv.setUint16(10, 0, true);
    cv.setUint16(12, dosTime, true);
    cv.setUint16(14, dosDate, true);
    cv.setUint32(16, crc, true);
    cv.setUint32(20, size, true);
    cv.setUint32(24, size, true);
    cv.setUint16(28, nameBytes.length, true);
    cv.setUint16(30, 0, true);
    cv.setUint16(32, 0, true);
    cv.setUint16(34, 0, true);
    cv.setUint16(36, 0, true);
    cv.setUint32(38, 0, true);
    cv.setUint32(42, offset, true);
    ch.set(nameBytes, 46);
    centrals.push(ch);

    offset += lh.length + size;
  }

  const cdStart = offset;
  let cdSize = 0;
  for (const c of centrals) cdSize += c.length;

  const eocd = new Uint8Array(22);
  const ev = new DataView(eocd.buffer);
  ev.setUint32(0, 0x06054b50, true);
  ev.setUint16(4, 0, true);
  ev.setUint16(6, 0, true);
  ev.setUint16(8, files.length, true);
  ev.setUint16(10, files.length, true);
  ev.setUint32(12, cdSize, true);
  ev.setUint32(16, cdStart, true);
  ev.setUint16(20, 0, true);

  const total = offset + cdSize + 22;
  const out = new Uint8Array(total);
  let p = 0;
  for (const part of locals) { out.set(part, p); p += part.length; }
  for (const c of centrals) { out.set(c, p); p += c.length; }
  out.set(eocd, p);
  return out;
}

// ---------- ld+json 抓作者/封面 ----------
function extractLdJson(html) {
  const out = {};
  const re = /<script[^>]+type="application\/ld\+json"[^>]*>([\s\S]*?)<\/script>/gi;
  let m;
  while ((m = re.exec(html))) {
    try {
      const j = JSON.parse(m[1]);
      const arr = Array.isArray(j) ? j : [j];
      for (const x of arr) {
        if (x.author && !out.author) {
          const a = Array.isArray(x.author) ? x.author[0] : x.author;
          out.author = a?.name || "";
        }
        if (x.image && !out.cover) {
          out.cover = Array.isArray(x.image) ? x.image[0] : x.image;
        }
        if (x.headline && !out.headline) out.headline = x.headline;
      }
    } catch (e) {}
  }
  return out;
}

const escHtml = (s) => String(s)
  .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
  .replace(/"/g, "&quot;");

function decodeFont(s) {
  let out = "";
  for (const ch of s) {
    const r = FONT_MAP[ch.codePointAt(0)];
    out += r !== undefined ? r : ch;
  }
  return out;
}

function parseState(html) {
  const marker = "window.__INITIAL_STATE__=";
  const i = html.indexOf(marker);
  if (i < 0) return null;
  const s = html.slice(i + marker.length).trimStart();
  let depth = 0, inStr = false, esc = false, end = -1;
  for (let k = 0; k < s.length; k++) {
    const c = s[k];
    if (inStr) {
      if (esc) esc = false;
      else if (c === "\\") esc = true;
      else if (c === '"') inStr = false;
      continue;
    }
    if (c === '"') { inStr = true; continue; }
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (depth === 0) { end = k + 1; break; } }
  }
  if (end < 0) return null;
  try { return JSON.parse(s.slice(0, end)); } catch (e) { return null; }
}

const stripTags = (h) => h
  .replace(/<script[\s\S]*?<\/script>/gi, "")
  .replace(/<style[\s\S]*?<\/style>/gi, "")
  .replace(/<br\s*\/?>/gi, "\n")
  .replace(/<\/p>/gi, "\n")
  .replace(/<[^>]+>/g, "")
  .replace(/&nbsp;/g, " ").replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"')
  .replace(/\n{3,}/g, "\n\n").trim();

async function proxyList(env) {
  const out = [];
  let user = env?.FANQIE_PROXY_API || null;
  if (!user && env?.FANQIE_KV) user = await env.FANQIE_KV.get("proxy_api");
  if (user) out.push({ name: "user", base: String(user).replace(/\/$/, "") });
  for (const p of HARDCODED_PROXIES) out.push(p);
  return out;
}

async function proxyGet(env, path, params) {
  const qs = new URLSearchParams(params).toString();
  for (const p of await proxyList(env)) {
    try {
      const r = await fetchT(`${p.base}${path}?${qs}`, { headers: { "user-agent": UA } });
      if (!r.ok) continue;
      const d = await r.json();
      if (d && d.code === 200) return { ok: true, from: p.name, data: d };
    } catch (e) {}
  }
  return { ok: false };
}

async function webGet(path) {
  const r = await fetchT(WEB_BASE + path, { headers: { "user-agent": UA, accept: "text/html,application/xhtml+xml", "accept-language": "zh-CN,zh;q=0.9" } });
  if (!r.ok) throw new Error(`web ${r.status} ${path}`);
  return r.text();
}

// ---------- 搜索 ----------
async function hSearch(env, key) {
  if (!key) return json({ code: 400, message: "key required" }, 400);
  const r = await proxyGet(env, "/search", { key, tab_type: 3 });
  if (r.ok) return json({ code: 200, from: r.from, data: r.data });
  // 网页搜索端点实测 /search?q= 返回 404 (SPA 不服务该路由)，仅当 key 是 book_id 时可用
  if (/^\d{15,}$/.test(key)) {
    return json({ code: 200, from: "id", data: { book_ids: [key] }, note: "web search unavailable, key treated as book_id" });
  }
  return json({ code: 501, message: "web search endpoint not available; configure FANQIE_PROXY_API or pass a numeric book_id" }, 501);
}

// ---------- 详情 ----------
async function hDetail(env, book_id) {
  book_id = await parseBookId(book_id);
  if (!book_id) return json({ code: 400, message: "book_id or book URL required" }, 400);
  const r = await proxyGet(env, "/detail", { book_id });
  if (r.ok) return json({ code: 200, from: r.from, data: r.data });
  try {
    const html = await webGet(`/page/${book_id}`);
    const st = parseState(html);
    const p = st?.page || {};
    if (!p.bookName) return json({ code: 404, from: "web", message: "book not found" }, 404);
    const ld = extractLdJson(html);
    return json({
      code: 200, from: "web",
      data: {
        book_id: String(p.bookId || book_id),
        book_name: p.bookName,
        author: p.author || ld.author || "",
        abstract: p.abstract || "",
        status: String(p.status ?? ""),
        word_number: p.wordNumber ?? 0,
        chapter_total: p.chapterTotal ?? (p.itemIds || []).length,
        last_chapter_title: p.lastChapterTitle || "",
        cover: ld.cover || "",
      },
    });
  } catch (e) { return json({ code: 502, message: String(e) }, 502); }
}

// ---------- 目录 ----------
async function hDirectory(env, book_id) {
  book_id = await parseBookId(book_id);
  if (!book_id) return json({ code: 400, message: "book_id or book URL required" }, 400);
  const r = await proxyGet(env, "/directory", { book_id });
  if (r.ok) return json({ code: 200, from: r.from, data: r.data });
  try {
    const html = await webGet(`/page/${book_id}`);
    const st = parseState(html);
    const p = st?.page || {};
    const ids = (p.itemIds || []).slice().reverse(); // 页面为倒序，反转成正序
    if (!ids.length) return json({ code: 404, from: "web", message: "no chapters" }, 404);
    // 标题：chapterList 只覆盖前 50 条，其余 title 留空由 /api/content 补
    const titleMap = {};
    for (const c of (p.chapterList || [])) {
      if (c.itemId) titleMap[String(c.itemId)] = c.title || "";
    }
    const lists = ids.map((id, i) => ({
      item_id: String(id),
      title: titleMap[String(id)] || "",
      order: i + 1,
    }));
    return json({
      code: 200, from: "web",
      data: {
        book_id: String(p.bookId || book_id),
        book_name: p.bookName || "",
        author: p.author || "",
        chapter_total: ids.length,
        lists,
      },
    });
  } catch (e) { return json({ code: 502, message: String(e) }, 502); }
}

// ---------- 正文 ----------
async function hContent(env, item_id) {
  item_id = await parseBookId(item_id);
  if (!item_id) return json({ code: 400, message: "item_id or reader URL required" }, 400);
  const r = await proxyGet(env, "/content", { tab: "小说", item_id });
  if (r.ok) return json({ code: 200, from: r.from, data: r.data });
  try {
    const html = await webGet(`/reader/${item_id}`);
    const st = parseState(html);
    const cd = st?.reader?.chapterData;
    if (!cd || !cd.content) return json({ code: 404, from: "web", message: "chapter not found or locked" }, 404);
    const raw = stripTags(cd.content);
    const content = decodeFont(raw);
    return json({
      code: 200, from: "web",
      data: {
        item_id: String(cd.itemId || item_id),
        book_id: String(cd.bookId || ""),
        book_name: cd.bookName || "",
        title: cd.title || "",
        order: cd.order ?? 0,
        word_count: cd.chapterWordNumber ?? 0,
        content,
      },
    });
  } catch (e) { return json({ code: 502, message: String(e) }, 502); }
}

// ---------- 分段下载 (规避 Workers 子请求上限) ----------
async function hDownload(env, book_id, start, count) {
  book_id = await parseBookId(book_id);
  if (!book_id) return json({ code: 400, message: "book_id or book URL required" }, 400);
  const dj = await (await hDirectory(env, book_id)).json();
  if (dj.code !== 200) return json(dj, dj.code || 500);
  const lists = dj?.data?.lists || [];
  const s = Math.max(0, parseInt(start || "0", 10) || 0);
  const n = Math.min(30, Math.max(1, parseInt(count || "20", 10) || 20));
  const slice = lists.slice(s, s + n);
  if (!slice.length) return json({ code: 416, message: "range out of bounds", total: lists.length }, 416);
  const parts = [];
  for (const it of slice) {
    const cj = await (await hContent(env, String(it.item_id))).json();
    const title = it.title || cj?.data?.title || it.item_id;
    parts.push(`## ${title}\n\n${cj?.data?.content || "[获取失败]"}`);
  }
  return json({
    code: 200,
    book_id, book_name: dj.data.book_name || "",
    total: lists.length, start: s, count: slice.length,
    next: s + slice.length < lists.length ? s + slice.length : null,
    text: parts.join("\n\n"),
  });
}


// ---------- 整本导出（txt / html / epub，分批） ----------
async function hExport(env, book_id, format, start, count) {
  book_id = await parseBookId(book_id);
  if (!book_id) return json({ code: 400, message: "book_id or book URL required" }, 400);
  format = (format || "txt").toLowerCase();
  if (!["txt", "html", "epub"].includes(format)) {
    return json({ code: 400, message: "format must be txt|html|epub" }, 400);
  }

  const dj = await (await hDirectory(env, book_id)).json();
  if (dj.code !== 200) return json(dj, dj.code || 500);
  const lists = dj.data.lists || [];
  const bookName = dj.data.book_name || book_id;
  const total = lists.length;

  const s = Math.max(0, parseInt(start || "0", 10) || 0);
  const n = Math.min(45, Math.max(1, parseInt(count || "20", 10) || 20));
  const slice = lists.slice(s, s + n);
  if (!slice.length) return json({ code: 416, message: "range out of bounds", total }, 416);

  const parts = await fetchChaptersParallel(env, slice, 15);
  const tail = s + slice.length < total ? s + slice.length : null;

  if (format === "txt") {
    return json({ code: 200, format, book_id, book_name: bookName, total, start: s, count: slice.length, next: tail, text: parts.join("\n\n") });
  }

  // html / epub：本批生成片段，客户端拼接或服务端最终打包
  if (format === "html") {
    const body = slice.map((it, i) => {
      const raw = parts[i] || "";
      const title = it.title || raw.split("\n")[0].replace(/^##\s*/, "") || it.item_id;
      const content = raw.replace(/^##[^\n]*\n+/, "").split("\n").map((l) => `<p>${escHtml(l)}</p>`).join("\n");
      return `<section id="c_${it.item_id}"><h2>${escHtml(title)}</h2>${content}</section>`;
    }).join("\n");
    return json({ code: 200, format, book_id, book_name: bookName, total, start: s, count: slice.length, next: tail, html: body });
  }

  // epub：返回本批 xhtml 片段，最终由 /api/export/final 或 DO 打包
  const chapters = slice.map((it, i) => {
    const raw = parts[i] || "";
    const title = it.title || it.item_id;
    const content = raw.replace(/^##[^\n]*\n+/, "").split("\n").map((l) => `<p>${escHtml(l)}</p>`).join("\n");
    return { id: it.item_id, title, xhtml: `<?xml version="1.0" encoding="utf-8"?>\n<!DOCTYPE html>\n<html xmlns="http://www.w3.org/1999/xhtml"><head><meta charset="utf-8"/><title>${escHtml(title)}</title></head><body><h2>${escHtml(title)}</h2>${content}</body></html>` };
  });
  return json({ code: 200, format, book_id, book_name: bookName, total, start: s, count: slice.length, next: tail, chapters });
}

// ---------- 更新检查（对比 KV 记录的章节数） ----------
async function hUpdateCheck(env, book_id) {
  book_id = await parseBookId(book_id);
  if (!book_id) return json({ code: 400, message: "book_id or book URL required" }, 400);

  const dj = await (await hDirectory(env, book_id)).json();
  if (dj.code !== 200) return json(dj, dj.code || 500);
  const total = dj.data.chapter_total;
  const bookName = dj.data.book_name;

  let lastSeen = 0;
  if (env?.FANQIE_KV) {
    const v = await env.FANQIE_KV.get(`record:${book_id}`);
    lastSeen = parseInt(v || "0", 10) || 0;
    if (total > lastSeen) await env.FANQIE_KV.put(`record:${book_id}`, String(total));
  }

  const newCount = lastSeen ? Math.max(0, total - lastSeen) : total;
  return json({
    code: 200, book_id, book_name: bookName,
    total, last_seen: lastSeen, new_chapters: newCount,
    has_kv: !!env?.FANQIE_KV,
    next_start: lastSeen || 0,
  });
}


// ---------- 分批打包 zip（txt/html） ----------
async function hExportZip(env, book_id, format, start, count) {
  book_id = await parseBookId(book_id);
  if (!book_id) return json({ code: 400, message: "book_id or book URL required" }, 400);
  format = (format || "txt").toLowerCase();
  if (!["txt", "html"].includes(format)) {
    return json({ code: 400, message: "format must be txt|html" }, 400);
  }

  const dj = await (await hDirectory(env, book_id)).json();
  if (dj.code !== 200) return json(dj, dj.code || 500);
  const lists = dj.data.lists || [];
  const bookName = dj.data.book_name || book_id;
  const total = lists.length;

  const s = Math.max(0, parseInt(start || "0", 10) || 0);
  const n = Math.min(40, Math.max(1, parseInt(count || "20", 10) || 20));
  const slice = lists.slice(s, s + n);
  if (!slice.length) return json({ code: 416, message: "range out of bounds", total }, 416);

  const parts = await fetchChaptersParallel(env, slice, 15);
  const tail = s + slice.length < total ? s + slice.length : null;
  const safe = bookName.replace(/[<>:"/\\|?*]/g, "_");

  const files = [];
  if (format === "txt") {
    const body = parts.join("\n\n");
    files.push({ name: `${safe}_${s + 1}-${s + slice.length}.txt`, data: `\uFEFF${body}` });
  } else {
    files.push({
      name: "index.html",
      data: `<!DOCTYPE html><html><head><meta charset="utf-8"><title>${escHtml(bookName)}</title></head><body><h1>${escHtml(bookName)}</h1><p>第 ${s + 1} - ${s + slice.length} 章 / 共 ${total} 章</p></body></html>`,
    });
    slice.forEach((it, i) => {
      const raw = parts[i] || "";
      const title = it.title || it.item_id;
      const content = raw.replace(/^##[^\n]*\n+/, "").split("\n").map((l) => `<p>${escHtml(l)}</p>`).join("\n");
      files.push({
        name: `chapter_${String(s + i + 1).padStart(5, "0")}_${it.item_id}.html`,
        data: `<!DOCTYPE html><html><head><meta charset="utf-8"><title>${escHtml(title)}</title></head><body><h2>${escHtml(title)}</h2>${content}</body></html>`,
      });
    });
  }

  const zip = makeZip(files);
  const fname = `${safe}_${s + 1}-${s + slice.length}.zip`;
  return new Response(zip, {
    headers: {
      "content-type": "application/zip",
      "content-disposition": `attachment; filename="${fname}"`,
      "x-total": String(total),
      "x-start": String(s),
      "x-count": String(slice.length),
      "x-next": tail === null ? "" : String(tail),
      ...CORS,
    },
  });
}

async function hConfig(req, env) {
  if (req.method === "GET") {
    let v = env?.FANQIE_PROXY_API || "";
    if (env?.FANQIE_KV) v = (await env.FANQIE_KV.get("proxy_api")) || v;
    return json({ code: 200, proxy_api: v, hardcoded: HARDCODED_PROXIES.length });
  }
  if (req.method === "POST") {
    if (!env?.FANQIE_KV) return json({ code: 500, message: "FANQIE_KV not bound" }, 500);
    const b = await req.json().catch(() => ({}));
    await env.FANQIE_KV.put("proxy_api", String(b.proxy_api || "").trim());
    return json({ code: 200 });
  }
  return json({ code: 405 }, 405);
}

// ---------- Telegram Bot ----------
const TG_API = "https://api.telegram.org/bot";

async function tgCall(token, method, body) {
  return fetch(`${TG_API}${token}/${method}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
}

async function tgSend(token, chatId, text) {
  return tgCall(token, "sendMessage", { chat_id: chatId, text, disable_web_page_preview: true });
}

async function tgSendDoc(token, chatId, filename, content, caption) {
  const form = new FormData();
  form.append("chat_id", String(chatId));
  if (caption) form.append("caption", caption);
  form.append("document", new Blob([content], { type: "text/plain; charset=utf-8" }), filename);
  return fetch(`${TG_API}${token}/sendDocument`, { method: "POST", body: form });
}

async function fetchChaptersParallel(env, items, concurrency) {
  const out = [];
  for (let i = 0; i < items.length; i += concurrency) {
    const batch = items.slice(i, i + concurrency);
    const rs = await Promise.all(batch.map(async (it) => {
      try {
        const cj = await (await hContent(env, String(it.item_id))).json();
        const title = it.title || cj?.data?.title || it.item_id;
        return `## ${title}\n\n${cj?.data?.content || "[获取失败]"}`;
      } catch (e) {
        return `## ${it.item_id}\n\n[获取失败: ${String(e)}]`;
      }
    }));
    out.push(...rs);
  }
  return out;
}

async function tgHandleUpdate(env, token, update, origin) {
  const msg = update.message || update.channel_post;
  if (!msg || !msg.text) return;
  const chatId = msg.chat.id;
  const text = msg.text.trim();

  if (text === "/start" || text === "/help") {
    await tgSend(token, chatId,
      "发送番茄小说链接或 book_id 即可下载。\n" +
      "例:\nhttps://fanqienovel.com/page/7143038691944959011\n" +
      "或直接发 7143038691944959011");
    return;
  }

  const bookId = parseBookId(text);
  if (!bookId) {
    await tgSend(token, chatId, "无法识别，请发送番茄小说链接或 15 位以上 book_id");
    return;
  }

  const dirJson = await (await hDirectory(env, bookId)).json();
  if (dirJson.code !== 200) {
    await tgSend(token, chatId, "获取目录失败: " + (dirJson.message || "未知错误"));
    return;
  }

  const lists = dirJson.data.lists || [];
  const bookName = dirJson.data.book_name || bookId;
  const total = lists.length;
  if (!total) { await tgSend(token, chatId, "目录为空"); return; }

  // 优先走 Durable Object 链式 alarm（秒级）
  if (env?.FANQIE_DO) {
    const id = env.FANQIE_DO.idFromName(`book:${bookId}:${chatId}`);
    const stub = env.FANQIE_DO.get(id);
    await stub.fetch("https://do/", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        chatId, bookId, bookName, total, lists, cursor: 0, chunks: [], createdAt: Date.now()
      }),
    });
    await tgSend(token, chatId,
      `《${bookName}》共 ${total} 章，已启动加速下载（约 ${Math.ceil(total / 45)} 轮 × 数秒），完成后自动发送全书 TXT。`);
    return;
  }

  // 有 KV 时走 Cron 全自动队列（分钟级）
  if (env?.FANQIE_KV) {
    const jobKey = `job:${bookId}:${chatId}`;
    await env.FANQIE_KV.put(jobKey, JSON.stringify({
      chatId, bookId, bookName, total, lists, cursor: 0, chunks: [], createdAt: Date.now()
    }));
    let pending = [];
    try { pending = JSON.parse((await env.FANQIE_KV.get("jobs:pending")) || "[]"); } catch (e) {}
    if (!pending.includes(jobKey)) pending.push(jobKey);
    await env.FANQIE_KV.put("jobs:pending", JSON.stringify(pending));
    const eta = Math.ceil(total / 45);
    await tgSend(token, chatId, `《${bookName}》共 ${total} 章，已加入队列。每分钟抓 45 章，预计约 ${eta} 分钟完成，完成后自动发送全书 TXT。`);
    return;
  }

  const LIMIT = 30; // 无 KV 时降级：单次 30 章
  const n = Math.min(total, LIMIT);
  await tgSend(token, chatId, `《${bookName}》共 ${total} 章，开始下载前 ${n} 章...`);

  const slice = lists.slice(0, n);
  const parts = await fetchChaptersParallel(env, slice, 15);
  let txt = parts.join("\n\n");
  const MAX_BYTES = 45 * 1024 * 1024;
  let truncated = false;
  if (new TextEncoder().encode(txt).length > MAX_BYTES) {
    txt = txt.slice(0, Math.floor(txt.length * MAX_BYTES / new TextEncoder().encode(txt).length));
    truncated = true;
  }
  const filename = total > n ? `${bookName}_1-${n}_of_${total}.txt` : `${bookName}.txt`;
  const caption = `第 1-${n} 章 / 共 ${total} 章` + (truncated ? "（已截断至 45MB）" : "");

  await tgSendDoc(token, chatId, filename, txt, caption);

  if (total > n) {
    await tgSend(token, chatId,
      `已发送前 ${n} 章。剩余 ${total - n} 章用分页接口继续:\n` +
      `${origin}/api/download?book_id=${bookId}&start=${n}&count=30\n` +
      `用返回的 next 循环拉取`);
  } else {
    await tgSend(token, chatId, `《${bookName}》全部 ${total} 章已发送`);
  }
}

async function hTelegramWebhook(request, env, ctx) {
  const token = env?.TG_BOT_TOKEN;
  if (!token) return json({ ok: false, error: "TG_BOT_TOKEN not set" }, 500);

  if (env?.TG_WEBHOOK_SECRET) {
    const hdr = request.headers.get("x-telegram-bot-api-secret-token");
    if (hdr !== env.TG_WEBHOOK_SECRET) return json({ ok: false }, 403);
  }

  let update;
  try { update = await request.json(); } catch (e) { return json({ ok: true }); }

  const origin = new URL(request.url).origin;
  ctx.waitUntil(tgHandleUpdate(env, token, update, origin).catch(() => {}));
  return json({ ok: true });
}

async function hTelegramSetup(request, env) {
  const token = env?.TG_BOT_TOKEN;
  if (!token) return json({ ok: false, error: "TG_BOT_TOKEN not set" }, 500);
  const origin = new URL(request.url).origin;
  const body = { url: `${origin}/tg/webhook` };
  if (env?.TG_WEBHOOK_SECRET) body.secret_token = env.TG_WEBHOOK_SECRET;
  const r = await fetch(`${TG_API}${token}/setWebhook`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  return json({ ok: r.ok, webhook: body.url, telegram: await r.json() });
}

// ---------- Cron 全自动队列 ----------
async function handleCron(env) {
  if (!env?.FANQIE_KV) return;
  const token = env?.TG_BOT_TOKEN;
  if (!token) return;

  let pending = [];
  try { pending = JSON.parse((await env.FANQIE_KV.get("jobs:pending")) || "[]"); } catch (e) {}
  if (!pending.length) return;

  const jobKey = pending[0];
  const jobRaw = await env.FANQIE_KV.get(jobKey);
  if (!jobRaw) {
    pending.shift();
    await env.FANQIE_KV.put("jobs:pending", JSON.stringify(pending));
    return;
  }

  let job;
  try { job = JSON.parse(jobRaw); } catch (e) {
    await env.FANQIE_KV.delete(jobKey);
    pending.shift();
    await env.FANQIE_KV.put("jobs:pending", JSON.stringify(pending));
    return;
  }

  const BATCH = 45;
  const start = job.cursor || 0;
  const end = Math.min(start + BATCH, job.total);
  const slice = job.lists.slice(start, end);

  const parts = await fetchChaptersParallel(env, slice, 10);
  job.chunks = job.chunks || [];
  job.chunks.push(...parts);
  job.cursor = end;

  if (job.cursor >= job.total) {
    let txt = job.chunks.join("\n\n");
    const MAX_BYTES = 45 * 1024 * 1024;
    if (new TextEncoder().encode(txt).length > MAX_BYTES) {
      txt = txt.slice(0, Math.floor(txt.length * MAX_BYTES / new TextEncoder().encode(txt).length));
    }
    await tgSendDoc(token, job.chatId, `${job.bookName}.txt`, txt, `全书 ${job.total} 章`);
    await tgSend(token, job.chatId, `《${job.bookName}》全部 ${job.total} 章已发送完毕`);
    await env.FANQIE_KV.delete(jobKey);
    pending.shift();
    await env.FANQIE_KV.put("jobs:pending", JSON.stringify(pending));
  } else {
    await env.FANQIE_KV.put(jobKey, JSON.stringify(job));
    const prev = Math.floor(start / job.total * 4);
    const now = Math.floor(job.cursor / job.total * 4);
    if (now > prev) {
      const pct = Math.floor(job.cursor / job.total * 100);
      await tgSend(token, job.chatId, `《${job.bookName}》进度 ${job.cursor}/${job.total} (${pct}%)`);
    }
  }
}


// ---------- Durable Object: 链式 alarm 加速 ----------
export class FanqieJob {
  constructor(state, env) {
    this.state = state;
    this.env = env;
  }

  async fetch(request) {
    const job = await request.json();
    job.chunks = job.chunks || [];
    await this.state.storage.put("job", job);
    await this.state.storage.setAlarm(Date.now() + 100);
    return new Response(JSON.stringify({ ok: true }), {
      headers: { "content-type": "application/json" },
    });
  }

  async alarm() {
    const job = await this.state.storage.get("job");
    if (!job) return;

    const token = this.env?.TG_BOT_TOKEN;
    const BATCH = 45;
    const start = job.cursor || 0;
    const end = Math.min(start + BATCH, job.total);
    const slice = job.lists.slice(start, end);

    let parts = [];
    try {
      parts = await fetchChaptersParallel(this.env, slice, 15);
    } catch (e) {
      parts = slice.map((it) => `## ${it.title || it.item_id}\n\n[获取失败: ${String(e)}]`);
    }

    job.chunks.push(...parts);
    job.cursor = end;

    if (job.cursor >= job.total) {
      let txt = job.chunks.join("\n\n");
      const MAX_BYTES = 45 * 1024 * 1024;
      let truncated = false;
      if (new TextEncoder().encode(txt).length > MAX_BYTES) {
        txt = txt.slice(0, Math.floor(txt.length * MAX_BYTES / new TextEncoder().encode(txt).length));
        truncated = true;
      }
      if (token) {
        await tgSendDoc(token, job.chatId, `${job.bookName}.txt`, txt,
          `全书 ${job.total} 章` + (truncated ? "（已截断至 45MB）" : ""));
        await tgSend(token, job.chatId, `《${job.bookName}》全部 ${job.total} 章已发送完毕`);
      }
      await this.state.storage.deleteAll();
      return;
    }

    await this.state.storage.put("job", job);

    const prev = Math.floor(start / job.total * 4);
    const now = Math.floor(job.cursor / job.total * 4);
    if (token && now > prev) {
      const pct = Math.floor(job.cursor / job.total * 100);
      await tgSend(token, job.chatId, `《${job.bookName}》进度 ${job.cursor}/${job.total} (${pct}%)`);
    }

    // 立即调度下一轮（1 秒后），不依赖 Cron
    await this.state.storage.setAlarm(Date.now() + 1000);
  }
}


export default {
  async fetch(request, env, ctx) {
    const u = new URL(request.url), p = u.pathname;
    if (request.method === "OPTIONS") return new Response(null, { headers: CORS });
    try {
      if (p === "/") return json({
        code: 200,
        endpoints: [
          "/api/search?key=",
          "/api/detail?book_id=",
          "/api/directory?book_id=",
          "/api/content?item_id=",
          "/api/download?book_id=&start=0&count=20",
          "/api/export?book_id=&format=txt|html|epub&start=0&count=20",
          "/api/export/zip?book_id=&format=txt|html&start=0&count=20",
          "/api/update?book_id=",
          "/api/config",
          "POST /tg/webhook",
          "GET /tg/setup",
          "CRON * * * * * (全自动队列)",
        ],
        font_map_entries: Object.keys(FONT_MAP).length,
      });
      if (p === "/api/search") return hSearch(env, u.searchParams.get("key") || "");
      if (p === "/api/detail") return hDetail(env, u.searchParams.get("book_id") || "");
      if (p === "/api/directory") return hDirectory(env, u.searchParams.get("book_id") || "");
      if (p === "/api/content") return hContent(env, u.searchParams.get("item_id") || "");
      if (p === "/api/download") return hDownload(env, u.searchParams.get("book_id") || "", u.searchParams.get("start"), u.searchParams.get("count"));
      if (p === "/api/export") return hExport(env, u.searchParams.get("book_id") || "", u.searchParams.get("format"), u.searchParams.get("start"), u.searchParams.get("count"));
      if (p === "/api/update") return hUpdateCheck(env, u.searchParams.get("book_id") || "");
      if (p === "/api/export/zip") return hExportZip(env, u.searchParams.get("book_id") || "", u.searchParams.get("format"), u.searchParams.get("start"), u.searchParams.get("count"));
      if (p === "/api/config") return hConfig(request, env);
      if (p === "/tg/webhook" && request.method === "POST") return hTelegramWebhook(request, env, ctx);
      if (p === "/tg/setup") return hTelegramSetup(request, env);
      return json({ code: 404 }, 404);
    } catch (e) { return json({ code: 500, message: String(e) }, 500); }
  },
  async scheduled(event, env, ctx) {
    ctx.waitUntil(handleCron(env));
  },
};
