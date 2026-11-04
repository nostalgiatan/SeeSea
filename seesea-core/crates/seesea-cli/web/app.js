/* SeeSea workbench. No framework, no build step; everything the server sends is put in the page as text. */
'use strict';

const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => [...r.querySelectorAll(s)];

const reduceMotion = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
/* Restart a CSS animation class on an element. */
function replay(el, cls, ms) {
  if (!el) return;
  el.classList.remove(cls); void el.offsetWidth; el.classList.add(cls);
  if (ms) setTimeout(() => el.classList.remove(cls), ms);
}
function moveInd() {
  const ind = $('#nav-ind'), on = $('#nav a.on');
  if (!ind) return;
  if (!on || !on.offsetHeight) { ind.style.opacity = '0'; return; }
  const h = Math.round(on.offsetHeight * .62);
  ind.style.height = h + 'px';
  ind.style.transform = `translateY(${on.offsetTop + Math.round((on.offsetHeight - h) / 2)}px)`;
  ind.style.opacity = '1';
}
addEventListener('resize', moveInd);

function h(tag, attrs, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v == null || v === false) continue;
    if (k === 'class') e.className = v;
    else if (k === 'text') e.textContent = v;
    // The page's CSP forbids style attributes; the CSSOM is allowed.
    else if (k === 'style') for (const decl of String(v).split(';')) { const i = decl.indexOf(':'); if (i > 0) e.style.setProperty(decl.slice(0, i).trim(), decl.slice(i + 1).trim()); }
    else if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else e.setAttribute(k, v === true ? '' : v);
  }
  for (const c of kids.flat(Infinity)) if (c != null && c !== false) e.append(c.nodeType ? c : document.createTextNode(String(c)));
  return e;
}

/* ---------- strings ---------- */
const D = {
  zh: {
    'nav.search': '搜索', 'nav.read': '阅读', 'nav.review': '复盘', 'nav.status': '状态', 'nav.services': '服务', 'nav.accounts': '账号',
    'search.ph': '搜点什么', 'search.go': '搜索',
    'mode.fast': '快', 'mode.standard': '标准', 'mode.deep': '深',
    'opt.time': '时间', 'opt.any': '不限', 'opt.day': '一天', 'opt.week': '一周', 'opt.month': '一月', 'opt.year': '一年',
    'opt.topic': '类型', 'opt.general': '综合', 'opt.news': '新闻', 'opt.limit': '条数', 'opt.include': '只看', 'opt.exclude': '排除',
    'opt.domains': '域名，逗号分隔', 'opt.full': '整页正文',
    'read.title': '阅读一个网页', 'read.sub': '抓取、抽取正文；给出问题时只留下回答它的章节。', 'read.go': '读取', 'read.query': '问题',
    'read.qph': '可选：这页怎么回答……', 'read.scope': '范围', 'read.core': '正文', 'read.ext': '含周边',
    'status.title': '状态', 'status.sub': '来源是否在线、缓存命中、本地索引。', 'status.sources': '来源', 'status.check': '逐个探测',
    'status.cache': '缓存', 'status.index': '本地索引',
    'svc.title': '服务', 'svc.sub': '从这个工作台启动 HTTP 接口和远程 MCP；它们和这里共用同一套模型与索引。',
    'acc.title': '账号', 'acc.sub': '服务用 “账号” 或 “两者” 认证时，这里的用户和密钥有效。',
    'key.title': '需要密钥', 'key.sub': '这个服务要求 API 密钥。它只保存在这个标签页里，关掉就没有了。', 'key.cancel': '取消', 'key.ok': '好',
    'sum.hits': '{n} 条结果', 'sum.cached': '缓存', 'sum.none': '没有找到结果',
    'hit.detail': '详情', 'hit.good': '好', 'hit.bad': '差', 'hit.open': '打开', 'hit.nottext': '没有读到正文',
    'd.score': '分数构成', 'd.sections': '读到的章节', 'd.related': '同一事件的其他报道', 'd.found': '来源', 'd.layer1': '第一层', 'd.layer2': '第二层',
    'fb.ok': '已记下：{h} {v}', 'fb.root': '只有根密钥可以标注', 'err': '出错了',
    'read.need': '先填一个网址', 'read.conf': '置信度', 'read.final': '最终网址', 'read.sections': '章节', 'read.asides': '被排除的块', 'read.render': '需要脚本渲染',
    'st.probe': '探测中…', 'st.none': '没有数据', 'st.hit': '命中', 'st.miss': '未命中', 'st.ratio': '命中率',
    'svc.api': 'HTTP 接口', 'svc.api.d': 'JSON 搜索接口，同时在 /mcp 提供 MCP。', 'svc.mcp': '远程 MCP', 'svc.mcp.d': '只提供 /mcp：给支持远程 MCP 的客户端用，不暴露其余接口。',
    'svc.running': '运行中', 'svc.stopped': '已停止', 'svc.start': '启动', 'svc.stop': '停止', 'svc.restart': '应用并重启', 'svc.save': '保存设置',
    'f.addr': '监听地址', 'f.auth': '谁可以调用', 'f.key': '共享密钥', 'f.cert': 'TLS 证书文件', 'f.tlskey': 'TLS 私钥文件', 'f.proxy': '前面有终止 TLS 的反向代理',
    'f.open': '允许无密钥监听非本机地址', 'f.conc': '并发上限', 'f.auto': '随工作台启动',
    'auth.open': '不要密钥（仅本机）', 'auth.key': '一个共享密钥', 'auth.accounts': '账号里的密钥', 'auth.both': '两者都行',
    'key.show': '显示', 'key.hide': '隐藏', 'key.copy': '复制', 'key.rotate': '换新', 'copied': '已复制',
    'svc.how': '怎么接入', 'svc.warn.net': '非本机地址需要 TLS（或声明前面有代理）和密钥。', 'svc.warn.open': '没有密钥而对外监听：除非勾选 “允许无密钥”，否则会被拒绝。',
    'svc.saved': '已保存', 'svc.started': '已启动', 'svc.stopped.t': '已停止', 'svc.placeholder': '<账号密钥>', 'svc.lan': '换成这台机器的地址',
    'acc.add': '添加用户', 'acc.name': '用户名', 'acc.plan': '套餐', 'acc.newkey': '新密钥', 'acc.disable': '停用', 'acc.enable': '启用', 'acc.revoke': '吊销',
    'acc.secret': '新密钥只显示这一次，请现在复制：', 'acc.today': '今日用量', 'acc.nokeys': '没有密钥', 'acc.none': '还没有用户',
    'acc.used': '最近使用', 'acc.expires': '到期', 'acc.never': '从未',
    'p.go': '前往', 'p.theme': '切换明暗', 'p.lang': '切换语言', 'p.key': '设置 API 密钥', 'p.focus': '聚焦搜索框', 'p.ph': '输入命令或页面名…',
    'where.desk': '工作台 · 本机', 'where.down': '连不上服务',
    'rv.title': '复盘', 'rv.sub': '看最近的搜索哪里不对，给结果打分，攒成评估用的标注。', 'rv.since': '范围', 'rv.24h': '最近 24 小时', 'rv.7d': '最近 7 天', 'rv.30d': '最近 30 天', 'rv.reload': '刷新',
    'rv.searches': '搜索', 'rv.cached': '命中缓存', 'rv.errors': '出错', 'rv.fb': '好 / 差', 'rv.src': '来源', 'rv.answered': '答了', 'rv.failed': '失败', 'rv.unread': '读不到的主机', 'rv.learn': '学到的主机评分',
    'rv.best': '最好', 'rv.worst': '最差', 'rv.sus': '值得看的搜索', 'rv.again': '再搜一次', 'rv.none': '这段时间没有记录', 'rv.nosus': '没有可疑的搜索', 'rv.grade': '给结果打分',
    'rv.scale': '3 正是想要的 · 2 回答了问题 · 1 相关 · 0 无关。点已选的分数可以取消。', 'rv.progress': '已标 {n} 条，其中 {q} 个查询可用于评估', 'rv.file': '标注文件', 'rv.run': '评估命令', 'rv.nocand': '没有可标的结果',
    'tip.palette': '命令面板  Ctrl/⌘ K', 'tip.lang': '切换语言  中 / EN', 'tip.theme': '切换明暗', 'tip.key': 'API 密钥',
    'rv.latency': '耗时 p50 / p95（毫秒）', 'read.pagetitle': '标题', 'read.rendered': '已渲染', 'read.yes': '是', 'acc.off': '已停用',
    'rv.learned': '{o} 次观察 · {h} 台主机', 'rv.obs': '{n} 次观察', 'st.fail': '失败', 'svc.auto': '— （自动生成）', 'svc.stdio': 'stdio（本机）',
    'stage.search': '检索', 'stage.fuse_rank': '融合排序', 'stage.fetch_extract': '抓取抽取', 'stage.rerank': '重排', 'stage.assemble': '组装',
    'brand.sub': '搜索数据中心', 'rail.help': '键盘', 'search.title': '搜索',
    'err.home': '回到搜索', 'err.retry': '重试', 'err.offline': '连不上服务', 'err.offline.d': '这个页面没能联系到后端。服务可能已停止，或网络断开；恢复后会自动重连。',
    'err.auth': '需要密钥', 'err.auth.d': '服务拒绝了这次请求。点右下角的钥匙图标输入 API 密钥。', 'err.nf': '页面不存在', 'err.nf.d': '没有 {p} 这个页面。',
    'err.srv': '服务出错了', 'err.gen': '出错了', 'err.na': '这个页面在当前服务上不可用', 'err.na.d': '服务、账号和复盘只在 seesea desk 里提供。',
    'cx.open': '在新标签页打开', 'cx.copylink': '复制链接', 'cx.copytitle': '复制标题', 'cx.copysel': '复制所选文字', 'cx.detail': '展开 / 收起详情',
    'cx.good': '标为好结果', 'cx.bad': '标为差结果', 'cx.reload': '刷新此页', 'cx.go': '前往', 'cx.palette': '命令面板', 'cx.sep': '',
    'where.up': '已重新连上',
    'grp.work': '工作', 'grp.watch': '观察', 'grp.api': '接口', 'crumb.root': '工作台',
    'sv.req': '请求', 'sv.ok': '成功', 'sv.err': '出错', 'sv.up': '运行时长', 'sv.overview': '设置', 'sv.console': '控制台', 'sv.log': '请求日志',
    'cs.send': '发送', 'cs.auth': '用哪个密钥', 'cs.none': '不带密钥', 'cs.shared': '这个服务的共享密钥', 'cs.custom': '我输入的密钥',
    'lg.time': '时间', 'lg.req': '请求', 'lg.status': '状态', 'lg.who': '账号', 'lg.none': '还没有请求。去控制台发一个，或用上面的命令调用。', 'lg.auto': '每 3 秒刷新；只记录方法、路径、状态和账号，不记录搜索词和密钥',
    'pl.title': '套餐', 'pl.min': '每分钟', 'pl.burst': '突发', 'pl.day': '每日额度', 'pl.conc': '并发', 'pl.deep': '深度搜索', 'pl.users': '用户数',
    'us.title': '用户', 'us.keys': '个密钥',
    'nk.title': '新建密钥', 'nk.name': '用途', 'nk.exp': '有效期', 'nk.never': '不过期', 'nk.7': '7 天', 'nk.30': '30 天', 'nk.90': '90 天', 'nk.365': '1 年', 'nk.make': '生成',
    'key.rotate.q': '换新后，旧的共享密钥立刻失效，用它的客户端需要更新。', 'acc.disable.q': '停用 {n} 之后，TA 的所有密钥都会被拒绝。可以随时启用。', 'acc.revoke.q': '吊销 {id}_… 后它立刻失效，不能恢复。', 'acc.revoked': '已吊销',
    'ins.empty': '选一条结果，在这里看分数构成、读到的章节和相关报道。',
  },
  en: {
    'nav.search': 'Search', 'nav.read': 'Read', 'nav.review': 'Review', 'nav.status': 'Status', 'nav.services': 'Services', 'nav.accounts': 'Accounts',
    'search.ph': 'Search for something', 'search.go': 'Search',
    'mode.fast': 'Fast', 'mode.standard': 'Standard', 'mode.deep': 'Deep',
    'opt.time': 'Time', 'opt.any': 'Any', 'opt.day': 'Day', 'opt.week': 'Week', 'opt.month': 'Month', 'opt.year': 'Year',
    'opt.topic': 'Topic', 'opt.general': 'General', 'opt.news': 'News', 'opt.limit': 'Results', 'opt.include': 'Only', 'opt.exclude': 'Exclude',
    'opt.domains': 'domains, comma separated', 'opt.full': 'Whole page',
    'read.title': 'Read a page', 'read.sub': 'Fetch it and extract the text; with a question, keep only the sections that answer it.', 'read.go': 'Read', 'read.query': 'Question',
    'read.qph': 'Optional: how does this page answer…', 'read.scope': 'Scope', 'read.core': 'Core', 'read.ext': 'Extended',
    'status.title': 'Status', 'status.sub': 'Which sources answer, cache hits, the local index.', 'status.sources': 'Sources', 'status.check': 'Probe each',
    'status.cache': 'Cache', 'status.index': 'Local index',
    'svc.title': 'Services', 'svc.sub': 'Start the HTTP API and a remote MCP server from this workbench; they share its models and index.',
    'acc.title': 'Accounts', 'acc.sub': 'These users and keys count when a service authenticates with “accounts” or “both”.',
    'key.title': 'A key is needed', 'key.sub': 'This server asks for an API key. It is kept in this tab only and gone when you close it.', 'key.cancel': 'Cancel', 'key.ok': 'OK',
    'sum.hits': '{n} results', 'sum.cached': 'cached', 'sum.none': 'Nothing found',
    'hit.detail': 'Details', 'hit.good': 'Good', 'hit.bad': 'Bad', 'hit.open': 'Open', 'hit.nottext': 'The page text was not read',
    'd.score': 'Score parts', 'd.sections': 'Sections read', 'd.related': 'Other reports of the same event', 'd.found': 'Found by', 'd.layer1': 'layer 1', 'd.layer2': 'layer 2',
    'fb.ok': 'Noted: {h} {v}', 'fb.root': 'Only the root key may judge hosts', 'err': 'Something went wrong',
    'read.need': 'Enter a URL first', 'read.conf': 'Confidence', 'read.final': 'Final URL', 'read.sections': 'Sections', 'read.asides': 'Blocks left out', 'read.render': 'Needs scripts to render',
    'st.probe': 'Probing…', 'st.none': 'No data', 'st.hit': 'hits', 'st.miss': 'misses', 'st.ratio': 'ratio',
    'svc.api': 'HTTP API', 'svc.api.d': 'The JSON search API; it also serves MCP at /mcp.', 'svc.mcp': 'Remote MCP', 'svc.mcp.d': 'Only /mcp, for clients that speak remote MCP; none of the rest of the API.',
    'svc.running': 'Running', 'svc.stopped': 'Stopped', 'svc.start': 'Start', 'svc.stop': 'Stop', 'svc.restart': 'Apply and restart', 'svc.save': 'Save settings',
    'f.addr': 'Listen address', 'f.auth': 'Who may call', 'f.key': 'Shared key', 'f.cert': 'TLS certificate file', 'f.tlskey': 'TLS private key file', 'f.proxy': 'A reverse proxy in front ends TLS',
    'f.open': 'Allow a non-local address with no key', 'f.conc': 'Concurrency limit', 'f.auto': 'Start with the workbench',
    'auth.open': 'No key (this machine only)', 'auth.key': 'One shared key', 'auth.accounts': 'Keys from accounts', 'auth.both': 'Either',
    'key.show': 'Show', 'key.hide': 'Hide', 'key.copy': 'Copy', 'key.rotate': 'New key', 'copied': 'Copied',
    'svc.how': 'How to connect', 'svc.warn.net': 'A non-local address needs TLS (or a declared proxy) and a key.', 'svc.warn.open': 'Listening outside with no key is refused unless “allow no key” is ticked.',
    'svc.saved': 'Saved', 'svc.started': 'Started', 'svc.stopped.t': 'Stopped', 'svc.placeholder': '<account key>', 'svc.lan': 'use this machine’s address instead',
    'acc.add': 'Add user', 'acc.name': 'User name', 'acc.plan': 'Plan', 'acc.newkey': 'New key', 'acc.disable': 'Disable', 'acc.enable': 'Enable', 'acc.revoke': 'Revoke',
    'acc.secret': 'The new key is shown once; copy it now:', 'acc.today': 'Used today', 'acc.nokeys': 'No keys', 'acc.none': 'No users yet',
    'acc.used': 'last used', 'acc.expires': 'expires', 'acc.never': 'never',
    'p.go': 'Go to', 'p.theme': 'Toggle light / dark', 'p.lang': 'Switch language', 'p.key': 'Set API key', 'p.focus': 'Focus the search box', 'p.ph': 'Type a command or page…',
    'where.desk': 'Workbench · this machine', 'where.down': 'Cannot reach the server',
    'rv.title': 'Review', 'rv.sub': 'See where recent searches went wrong, grade results, and build judgments for evaluation.', 'rv.since': 'Range', 'rv.24h': 'Last 24 hours', 'rv.7d': 'Last 7 days', 'rv.30d': 'Last 30 days', 'rv.reload': 'Refresh',
    'rv.searches': 'Searches', 'rv.cached': 'Cached', 'rv.errors': 'Errors', 'rv.fb': 'Good / bad', 'rv.src': 'Sources', 'rv.answered': 'answered', 'rv.failed': 'failed', 'rv.unread': 'Hosts that could not be read', 'rv.learn': 'Learned host standings',
    'rv.best': 'Best', 'rv.worst': 'Worst', 'rv.sus': 'Searches worth a look', 'rv.again': 'Search again', 'rv.none': 'Nothing recorded in this range', 'rv.nosus': 'No suspicious searches', 'rv.grade': 'Grade results',
    'rv.scale': '3 exactly what was wanted · 2 answers it · 1 related · 0 not relevant. Click the chosen grade to take it back.', 'rv.progress': '{n} graded, {q} queries usable for evaluation', 'rv.file': 'Judgments file', 'rv.run': 'Evaluate with', 'rv.nocand': 'No results to grade',
    'tip.palette': 'Command palette  Ctrl/⌘ K', 'tip.lang': 'Switch language  中 / EN', 'tip.theme': 'Toggle light / dark', 'tip.key': 'API key',
    'rv.latency': 'Latency p50 / p95 (ms)', 'read.pagetitle': 'Title', 'read.rendered': 'Rendered', 'read.yes': 'yes', 'acc.off': 'Off',
    'rv.learned': '{o} observations · {h} hosts', 'rv.obs': '{n} observations', 'st.fail': 'failed', 'svc.auto': '— (generated)', 'svc.stdio': 'stdio (local)',
    'stage.search': 'Search', 'stage.fuse_rank': 'Fuse and rank', 'stage.fetch_extract': 'Fetch and extract', 'stage.rerank': 'Rerank', 'stage.assemble': 'Assemble',
    'brand.sub': 'Search data center', 'search.title': 'Search',
    'err.home': 'Back to search', 'err.retry': 'Retry', 'err.offline': 'Cannot reach the server', 'err.offline.d': 'The page could not talk to its back end. The service may have stopped or the network dropped; it reconnects on its own.',
    'err.auth': 'A key is needed', 'err.auth.d': 'The server refused this request. Use the key icon at the bottom of the rail to enter an API key.', 'err.nf': 'Page not found', 'err.nf.d': 'There is no page at {p}.',
    'err.srv': 'The server failed', 'err.gen': 'Something went wrong', 'err.na': 'This page is not available on this server', 'err.na.d': 'Services, accounts and review exist only in seesea desk.',
    'cx.open': 'Open in a new tab', 'cx.copylink': 'Copy link', 'cx.copytitle': 'Copy title', 'cx.copysel': 'Copy selected text', 'cx.detail': 'Expand / collapse details',
    'cx.good': 'Mark as good', 'cx.bad': 'Mark as bad', 'cx.reload': 'Reload this page', 'cx.go': 'Go to', 'cx.palette': 'Command palette', 'cx.sep': '',
    'where.up': 'Reconnected',
    'grp.work': 'Work', 'grp.watch': 'Watch', 'grp.api': 'API', 'crumb.root': 'Workbench',
    'sv.req': 'Requests', 'sv.ok': 'Succeeded', 'sv.err': 'Errors', 'sv.up': 'Uptime', 'sv.overview': 'Settings', 'sv.console': 'Console', 'sv.log': 'Request log',
    'cs.send': 'Send', 'cs.auth': 'Which key', 'cs.none': 'No key', 'cs.shared': 'This service’s shared key', 'cs.custom': 'A key I type',
    'lg.time': 'Time', 'lg.req': 'Request', 'lg.status': 'Status', 'lg.who': 'Account', 'lg.none': 'No requests yet. Send one from the console, or call it with a command above.', 'lg.auto': 'Refreshes every 3 s; only method, path, status and account are kept, never the query or the key',
    'pl.title': 'Plans', 'pl.min': 'Per minute', 'pl.burst': 'Burst', 'pl.day': 'Daily credits', 'pl.conc': 'Concurrent', 'pl.deep': 'Deep search', 'pl.users': 'Users',
    'us.title': 'Users', 'us.keys': 'keys',
    'nk.title': 'New key', 'nk.name': 'Used for', 'nk.exp': 'Expires', 'nk.never': 'Never', 'nk.7': '7 days', 'nk.30': '30 days', 'nk.90': '90 days', 'nk.365': '1 year', 'nk.make': 'Create',
    'key.rotate.q': 'Once replaced, the old shared key stops working at once; clients using it need the new one.', 'acc.disable.q': 'After disabling {n}, every key of theirs is refused. You can enable them again.', 'acc.revoke.q': '{id}_… stops working at once and cannot be restored.', 'acc.revoked': 'Revoked',
    'ins.empty': 'Select a result to see its score parts, the sections read and related reports here.',
  },
};

const S = { lang: 'zh', desk: false, mode: 'standard', hits: [], sel: -1, marks: {}, health: null };
const store = {
  get(k) { try { return localStorage.getItem(k); } catch { return null; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch { /* private mode */ } },
};
const session = {
  get(k) { try { return sessionStorage.getItem(k); } catch { return null; } },
  set(k, v) { try { v ? sessionStorage.setItem(k, v) : sessionStorage.removeItem(k); } catch { /* ignore */ } },
};
const t = (k, vars) => {
  let s = (D[S.lang] && D[S.lang][k]) ?? D.en[k] ?? k;
  for (const [a, b] of Object.entries(vars || {})) s = s.replace(`{${a}}`, b);
  return s;
};

function applyLang() {
  document.documentElement.lang = S.lang === 'zh' ? 'zh-CN' : 'en';
  for (const e of $$('[data-i]')) e.textContent = t(e.dataset.i);
  const other = S.lang === 'zh' ? 'en' : 'zh';
  for (const e of $$('[data-o]')) e.textContent = D[other][e.dataset.o] || '';
  for (const e of $$('[data-ip]')) e.placeholder = t(e.dataset.ip);
  for (const e of $$('[data-t]')) e.title = t(e.dataset.t);
  document.title = S.lang === 'zh' ? 'SeeSea 工作台' : 'SeeSea Workbench';
  updateChip();
  if (S.view && $('#crumb') && S.view !== 'error') $('#crumb').textContent = t('nav.' + S.view);
}
function themeSwitch(change, from) {
  if (!document.startViewTransition || reduceMotion()) return change();
  const r = from ? from.getBoundingClientRect() : { left: innerWidth - 40, top: 26, width: 0, height: 0 };
  const x = r.left + r.width / 2, y = r.top + r.height / 2, rad = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));
  const vt = document.startViewTransition(change);
  vt.ready.then(() => document.documentElement.animate({ clipPath: [`circle(0 at ${x}px ${y}px)`, `circle(${rad}px at ${x}px ${y}px)`] }, { duration: 520, easing: 'cubic-bezier(.3,.7,.2,1)', pseudoElement: '::view-transition-new(root)' })).catch(() => {});
}
function applyTheme(next) {
  const root = document.documentElement;
  if (next) { root.dataset.theme = next; store.set('ss_theme', next); }
  else if (store.get('ss_theme')) root.dataset.theme = store.get('ss_theme');
}
function toggleTheme(e) {
  const from = e && e.currentTarget && e.currentTarget.id === 'btn-theme' ? e.currentTarget : $('#btn-theme');
  themeSwitch(() => flipTheme(), from);
}
function flipTheme() {
  const dark = document.documentElement.dataset.theme
    ? document.documentElement.dataset.theme === 'dark'
    : matchMedia('(prefers-color-scheme: dark)').matches;
  applyTheme(dark ? 'light' : 'dark');
}

/* ---------- plumbing ---------- */
function toast(msg, bad) {
  const el = h('div', { class: 'toast' + (bad ? ' err' : ''), text: msg });
  $('#toasts').append(el);
  const life = bad ? 6000 : 2600;
  setTimeout(() => el.classList.add('out'), life - 240);
  setTimeout(() => el.remove(), life);
}
function errBox(e, retry) {
  const st = e.status || 0;
  const [title, desc] = e.offline ? [t('err.offline'), t('err.offline.d')]
    : st === 401 || st === 403 ? [t('err.auth'), t('err.auth.d')]
    : st === 404 ? [t('err.nf'), e.message]
    : st >= 500 ? [t('err.srv'), e.message] : [t('err.gen'), e.message];
  return h('div', { class: 'errbox' }, h('b', { class: 'code', text: e.offline ? '···' : st || '!' }),
    h('div', {}, h('strong', { text: title }), h('p', { text: desc }), retry ? h('button', { class: 'quiet', type: 'button', text: t('err.retry'), onclick: retry }) : null));
}
let offline = false;
function markOffline(v) {
  if (v === offline) return;
  offline = v;
  if (v) S.health = null; else api('/health', { noAuth: true }).then((x) => { S.health = x; updateChip(); if (S.recovered) toast(t('where.up')); }, () => {});
  if (v) S.recovered = true;
  updateChip();
}
const fail = (e) => toast(e && e.message ? e.message : t('err'), true);

function askKey() {
  const dlg = $('#dlg-key');
  if (dlg.open) return Promise.resolve(null);
  $('#key-in').value = '';
  return new Promise((resolve) => {
    dlg.addEventListener('close', () => resolve(dlg.returnValue === 'ok' ? $('#key-in').value.trim() || null : null), { once: true });
    dlg.showModal();
    $('#key-in').focus();
  });
}

async function api(path, { method = 'GET', body, noAuth } = {}) {
  const headers = {};
  // A local server answers a POST only when it says it is JSON, even an empty one.
  if (method !== 'GET' && body === undefined) body = {};
  const key = session.get('ss_key');
  if (key) headers.Authorization = 'Bearer ' + key;
  if (body !== undefined) headers['Content-Type'] = 'application/json';
  let res;
  try { res = await fetch(path, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) }); } catch {
    const e = new Error(t('err.offline')); e.offline = true; markOffline(true); throw e;
  }
  markOffline(false);
  if (res.status === 401 && !noAuth) {
    const k = await askKey();
    if (k) { session.set('ss_key', k); return api(path, { method, body, noAuth: true }); }
  }
  const text = await res.text();
  let data = null;
  if (text) { try { data = JSON.parse(text); } catch { /* not JSON */ } }
  if (!res.ok) {
    const e = new Error((data && data.message) || res.statusText || String(res.status));
    e.status = res.status; e.code = data && data.code; e.data = data;
    throw e;
  }
  return data;
}

function copy(text) {
  const btn = document.activeElement && document.activeElement.closest ? document.activeElement.closest('button') : null;
  navigator.clipboard.writeText(text).then(() => {
    toast(t('copied'));
    const use = btn && $('use', btn);
    if (use) { const was = use.getAttribute('href'); use.setAttribute('href', '#i-check'); btn.classList.add('copied'); setTimeout(() => { use.setAttribute('href', was); btn.classList.remove('copied'); }, 1300); }
  }, () => toast(text, true));
}

/* ---------- small renderers ---------- */
function hostOf(u) { try { return new URL(u).hostname.replace(/^www\./, ''); } catch { return ''; } }
function safeHref(u) { try { const x = new URL(u); return x.protocol === 'http:' || x.protocol === 'https:' ? x.href : null; } catch { return null; } }
function plain(md) {
  return String(md || '').replace(/```[\s\S]*?```/g, ' ').replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/[`*_>#]+/g, '').replace(/\s+/g, ' ').trim();
}
function ago(v) {
  if (v == null) return '';
  let d = typeof v === 'number' ? new Date(v < 1e12 ? v * 1000 : v) : new Date(v);
  if (isNaN(d)) return '';
  const s = (d - Date.now()) / 1000;
  const rtf = new Intl.RelativeTimeFormat(S.lang === 'zh' ? 'zh' : 'en', { numeric: 'auto' });
  for (const [unit, secs] of [['year', 31536000], ['month', 2592000], ['day', 86400], ['hour', 3600], ['minute', 60]]) {
    if (Math.abs(s) >= secs) return rtf.format(Math.round(s / secs), unit);
  }
  return rtf.format(Math.round(s), 'second');
}

function inline(text) {
  const out = [];
  const re = /`([^`]+)`|\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)|\*\*([^*]+)\*\*/g;
  let last = 0, m;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push(text.slice(last, m.index));
    if (m[1]) out.push(h('code', { text: m[1] }));
    else if (m[2]) out.push(h('a', { href: safeHref(m[3]) || '#', target: '_blank', rel: 'noopener noreferrer', text: m[2] }));
    else out.push(h('strong', { text: m[4] }));
    last = re.lastIndex;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}
function renderMd(md) {
  const box = h('div', { class: 'doc' });
  let para = [], code = null;
  const flush = () => { if (para.length) { box.append(h('pre', {}, inline(para.join('\n')))); para = []; } };
  for (const line of String(md || '').split('\n')) {
    if (/^```/.test(line)) {
      if (code) { box.append(h('pre', { class: 'code', text: code.join('\n') })); code = null; } else { flush(); code = []; }
      continue;
    }
    if (code) { code.push(line); continue; }
    const m = /^#{1,6}\s+(.*)/.exec(line);
    if (m) { flush(); box.append(h('h5', {}, inline(m[1]))); continue; }
    if (!line.trim()) { flush(); continue; }
    para.push(line);
  }
  flush();
  if (code) box.append(h('pre', { class: 'code', text: code.join('\n') }));
  return box;
}
function kvs(obj, prefix = '') {
  const dl = h('dl', { class: 'kv' });
  const walk = (o, p) => {
    for (const [k, v] of Object.entries(o || {})) {
      if (v && typeof v === 'object' && !Array.isArray(v)) walk(v, p + k + '.');
      else dl.append(h('dt', { text: p + k }), h('dd', { text: Array.isArray(v) ? v.join(', ') : String(v) }));
    }
  };
  walk(obj, prefix);
  return dl;
}

/* ---------- search ---------- */
const STAGES = ['search', 'fuse_rank', 'fetch_extract', 'rerank', 'assemble'];
const STAGE_COLOR = Object.fromEntries(STAGES.map((k, i) => [k, `var(--s${i})`]));
const stageName = (k) => t('stage.' + k);
const csv = (s) => s.split(',').map((x) => x.trim()).filter(Boolean);

async function doSearch() {
  const q = $('#q').value.trim();
  if (!q) return;
  const body = { query: q, mode: S.mode, limit: Math.max(1, Math.min(50, +$('#o-limit').value || 10)) };
  if ($('#o-time').value) body.time_range = $('#o-time').value;
  if ($('#o-topic').value !== 'general') body.topic = $('#o-topic').value;
  if (csv($('#o-inc').value).length) body.include_domains = csv($('#o-inc').value);
  if (csv($('#o-exc').value).length) body.exclude_domains = csv($('#o-exc').value);
  if ($('#o-full').checked) body.full_content = true;
  const out = $('#search-out'), go = $('#search-form .go');
  go.disabled = true;
  out.replaceChildren(h('div', { class: 'wait' }, h('div'), h('div'), h('div')));
  const t0 = performance.now();
  try {
    const r = await api('/v1/search', { method: 'POST', body });
    renderSearch(r, performance.now() - t0);
  } catch (e) {
    out.replaceChildren(errBox(e, doSearch), h('div', { class: 'notes' }, ...((e.data && e.data.failures) || []).map((f) => h('div', { class: 'note err', text: `${f.source}: ${f.kind} ${f.message}` }))));
  } finally { go.disabled = false; }
}

function renderSearch(r, wall) {
  const out = $('#search-out');
  S.last = [r, wall];
  S.hits = r.hits || [];
  S.sel = -1;
  showInspector(-1);
  const kids = [];
  const sum = h('div', { class: 'sum' }, h('span', {}, h('b', { text: t('sum.hits', { n: S.hits.length }) })), h('span', { text: `${r.took_ms ?? Math.round(wall)} ms` }), h('span', { class: 'tag', text: D[S.lang]['mode.' + r.mode] ? t('mode.' + r.mode) : r.mode }));
  if (r.cached) sum.append(h('span', { class: 'tag', text: t('sum.cached') }));
  kids.push(sum);
  const stages = (r.stages || []).filter((s) => STAGE_COLOR[s.stage]);
  if (stages.length) {
    kids.push(h('div', { class: 'voyage', title: (r.stages || []).map((s) => `${stageName(s.stage)} ${s.ms} ms`).join('\n') },
      stages.map((s) => h('div', { style: `flex-grow:${Math.max(s.ms, 1)};--c:${STAGE_COLOR[s.stage]}` }))));
    kids.push(h('div', { class: 'legend' }, stages.map((s) => h('span', { style: `--c:${STAGE_COLOR[s.stage]}`, text: `${stageName(s.stage)} ${s.ms} ms` }))));
  }
  const notes = h('div', { class: 'notes' });
  for (const d of r.degraded || []) notes.append(h('div', { class: 'note', text: d }));
  for (const f of r.failures || []) notes.append(h('div', { class: 'note err', text: `${f.source}: ${f.kind} ${f.message}` }));
  if (notes.children.length) kids.push(notes);
  if (!S.hits.length) kids.push(h('div', { class: 'empty' }, h('div', { class: 'big', text: t('sum.none') })));
  else {
    const max = Math.max(...S.hits.map((x) => x.score || 0), 1e-9);
    kids.push(h('div', { class: 'hits' }, S.hits.map((x, i) => hitCard(x, i, max))));
  }
  out.replaceChildren(...kids);
}

function hitCard(x, i, max) {
  const host = hostOf(x.url);
  const sec = x.content && x.content.sections && x.content.sections[0];
  const text = plain(sec ? sec.markdown : x.snippet).slice(0, 460) || plain(x.snippet);
  const href = safeHref(x.url);
  const card = h('article', { class: 'hit', style: `animation-delay:${Math.min(i, 12) * 28}ms`, 'data-n': i });
  card.append(
    h('div', { class: 'gauge' }, h('span', { class: 'n', text: String(i + 1).padStart(2, '0') }), h('div', { class: 'col', title: String(x.score) }, h('i', { style: `top:${100 - Math.max(6, ((x.score || 0) / max) * 100)}%` }))),
    h('div', { class: 'body' },
      h('h3', {}, h('span', { class: 'mono', text: (host[0] || '·') }), href ? h('a', { href, target: '_blank', rel: 'noopener noreferrer', text: x.title || x.url }) : (x.title || x.url)),
      h('div', { class: 'where' }, host, h('em', { text: (() => { try { const u = new URL(x.url); return (u.pathname + u.search).slice(0, 70); } catch { return ''; } })() }), x.published ? h('em', { text: '  ·  ' + ago(x.published) }) : null),
      text ? h('p', { class: 'snip', text }) : h('p', { class: 'snip', text: x.content_error || t('hit.nottext') }),
      h('div', { class: 'row' },
        (x.found_by || []).map((f) => h('span', { class: 'src' }, f.source, h('b', { text: '#' + f.rank }))),
        h('span', { class: 'sp' }),
        h('button', { class: 'act good', type: 'button', onclick: () => judge(i, 'good') }, ...withIcon('up', t('hit.good'))),
        h('button', { class: 'act bad', type: 'button', onclick: () => judge(i, 'bad') }, ...withIcon('down', t('hit.bad'))),
        h('button', { class: 'act', type: 'button', text: t('hit.detail'), onclick: () => toggleDetail(i) }))),
  );
  card.addEventListener('click', (e) => { if (!e.target.closest('a,button')) select(i); });
  return card;
}

const wide = () => matchMedia('(min-width: 1240px)').matches;

function buildDetail(i) {
  const x = S.hits[i], p = x.score_parts || {};
  const row = (k, layer2) => h('div', { class: 'part' + (layer2 ? ' l2' : '') }, h('span', { text: k }), h('div', { class: 't' }, h('i', { style: `width:${Math.min(100, Math.max(0, (p[k] || 0) * 100))}%` })), h('span', { text: (p[k] ?? 0).toFixed(3) }));
  const l1 = ['fusion', 'snippet_overlap', 'title', 'freshness', 'learned', 'stage1'].filter((k) => k in p);
  const l2 = ['relevance', 'title_support', 'body_quality'].filter((k) => k in p);
  const d = h('div', { class: 'detail' },
    h('div', {}, h('h4', { text: `${t('d.score')} · ${(x.score || 0).toFixed(3)}` }), h('div', { class: 'parts' }, l1.map((k) => row(k, false)), l2.map((k) => row(k, true)))));
  const secs = (x.content && x.content.sections) || [];
  if (secs.length) d.append(h('div', {}, h('h4', { text: t('d.sections') }), secs.map((s) => h('div', { class: 'sec' }, h('div', { class: 'path', text: `${(s.path || []).join(' › ')}  ·  ${(s.score ?? 0).toFixed(2)}` }), renderMd(s.markdown)))));
  if (x.content && x.content.markdown) d.append(h('div', {}, renderMd(x.content.markdown)));
  if ((x.related || []).length) d.append(h('div', { class: 'rel' }, h('h4', { text: t('d.related') }), x.related.map((r) => h('a', { href: safeHref(r.url) || '#', target: '_blank', rel: 'noopener noreferrer', text: `${r.host || hostOf(r.url)} — ${r.title}` }))));
  return d;
}

function showInspector(i) {
  const box = $('#inspector'), has = S.hits.length > 0;
  box.hidden = !has; $('.cols').classList.toggle('noins', !has);
  const x = S.hits[i];
  if (!x) { box.replaceChildren(h('div', { class: 'ins-empty' }, icon('search'), h('div', { text: t('ins.empty') }))); return; }
  const href = safeHref(x.url);
  replay(box, 'swap', 400);
  box.replaceChildren(
    h('div', { class: 'ins-h' }, h('h3', { text: x.title || x.url }), h('div', { class: 'where', text: hostOf(x.url) + (x.published ? '  ·  ' + ago(x.published) : '') }),
      h('div', { class: 'ins-act' },
        href ? h('a', { class: 'act', href, target: '_blank', rel: 'noopener noreferrer' }, ...withIcon('external', t('hit.open'))) : null,
        h('button', { class: 'act good', type: 'button', onclick: () => judge(i, 'good') }, ...withIcon('up', t('hit.good'))),
        h('button', { class: 'act bad', type: 'button', onclick: () => judge(i, 'bad') }, ...withIcon('down', t('hit.bad'))))),
    h('div', { class: 'ins-b' }, buildDetail(i)));
}

function toggleDetail(i) {
  if (wide()) { select(i); return; }
  const card = $(`.hit[data-n="${i}"]`);
  if (!card) return;
  const old = $('.detail', card);
  if (old) { old.remove(); return; }
  $('.body', card).after(buildDetail(i));
}

function select(i) {
  const cards = $$('.hit');
  if (!cards.length) return;
  S.sel = Math.max(0, Math.min(cards.length - 1, i));
  cards.forEach((c, n) => c.classList.toggle('sel', n === S.sel));
  cards[S.sel].scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  if (wide()) showInspector(S.sel);
}

async function judge(i, verdict) {
  const x = S.hits[i];
  try {
    const r = await api('/v1/feedback', { method: 'POST', body: { url: x.url, verdict } });
    for (const root of [$(`.hit[data-n="${i}"]`), S.sel === i ? $('#inspector') : null]) {
      if (!root) continue;
      $('.good', root).classList.toggle('on', verdict === 'good');
      $('.bad', root).classList.toggle('on', verdict === 'bad');
    }
    toast(t('fb.ok', { h: r.host, v: verdict }));
  } catch (e) { e.status === 403 ? toast(t('fb.root'), true) : fail(e); }
}

/* ---------- read ---------- */
async function doRead() {
  const url = $('#r-url').value.trim();
  if (!url) return toast(t('read.need'), true);
  const body = { url, scope: $('#r-scope').value };
  if ($('#r-q').value.trim()) body.query = $('#r-q').value.trim();
  const out = $('#read-out'), go = $('#read-form .go');
  go.disabled = true;
  out.replaceChildren(h('div', { class: 'wait' }, h('div'), h('div')));
  try {
    renderRead(await api('/v1/extract', { method: 'POST', body }));
  } catch (e) {
    out.replaceChildren(errBox(e, doRead));
  } finally { go.disabled = false; }
}

function renderRead(r) {
  S.lastRead = r;
  const meta = r.meta || {};
  const dl = h('dl', { class: 'meta' });
  const add = (k, v) => { if (v != null && v !== '') dl.append(h('dt', { text: k }), h('dd', { text: String(v) })); };
  add(t('read.pagetitle'), meta.title); add(t('read.final'), r.final_url); add(t('read.conf'), r.confidence != null ? Number(r.confidence).toFixed(2) : null);
  add(t('read.render'), r.needs_render ? (r.rendered ? t('read.rendered') : t('read.yes')) : null); add(t('read.asides'), (r.asides || []).length || null);
  const kids = [h('div', { class: 'panel' }, dl)];
  if (r.sections && r.sections.length) kids.push(h('div', { class: 'panel gap' }, r.sections.map((s) => h('div', { class: 'sec' }, h('div', { class: 'path', text: `${(s.path || []).join(' › ')}  ·  ${(s.score ?? 0).toFixed(2)}` }), renderMd(s.markdown)))));
  else if (r.markdown) kids.push(h('div', { class: 'panel gap' }, renderMd(r.markdown)));
  $('#read-out').replaceChildren(...kids);
}

/* ---------- status ---------- */
async function loadStatus() {
  const srcBox = $('#st-sources'), cacheBox = $('#st-cache'), idxBox = $('#st-index');
  try {
    const s = await api('/v1/sources');
    srcBox.replaceChildren(...(s.sources || []).map((n) => h('div', { class: 'lat' }, h('span', { class: 'lamp' }), h('span', { text: n }), h('span'), h('span'))));
  } catch (e) { srcBox.replaceChildren(errBox(e, loadStatus)); }
  try {
    const c = (await api('/v1/stats')).cache || {};
    const rows = Object.entries(c);
    cacheBox.replaceChildren(rows.length ? h('table', {}, h('tr', {}, h('th', { text: '' }), h('th', { class: 'r', text: t('st.hit') }), h('th', { class: 'r', text: t('st.miss') }), h('th', { class: 'r', text: t('st.ratio') })),
      rows.map(([n, v]) => h('tr', {}, h('td', { text: n }), h('td', { class: 'r', text: v.hits }), h('td', { class: 'r', text: v.misses }), h('td', { class: 'r', text: (v.hit_ratio * 100).toFixed(0) + '%' })))) : h('div', { class: 'mut', text: t('st.none') }));
  } catch (e) { cacheBox.replaceChildren(e.offline ? h('div', { class: 'mut', text: t('st.none') }) : errBox(e, loadStatus)); }
  try { idxBox.replaceChildren(kvs(await api('/v1/index'))); } catch (e) { idxBox.replaceChildren(e.offline ? h('div', { class: 'mut', text: t('st.none') }) : errBox(e, loadStatus)); }
}
async function probe() {
  const box = $('#st-sources');
  box.replaceChildren(h('div', { class: 'mut', text: t('st.probe') }));
  try {
    const r = (await api('/v1/sources/check')).sources || [];
    const max = Math.max(...r.map((x) => x.ms || 0), 1);
    box.replaceChildren(...r.map((x) => h('div', { class: 'lat', title: x.error || '' },
      h('span', { class: 'lamp ' + (x.ok ? 'ok' : 'no') }), h('span', { text: x.source }), h('div', { class: 't' }, h('i', { style: `width:${((x.ms || 0) / max) * 100}%` })), h('code', { text: x.ok ? `${x.ms} ms` : t('st.fail') }))));
  } catch (e) { box.replaceChildren(errBox(e, probe)); }
}

/* ---------- services (workbench only) ---------- */
const SVGNS = 'http://www.w3.org/2000/svg';
function icon(name, cls) {
  const s = document.createElementNS(SVGNS, 'svg'), u = document.createElementNS(SVGNS, 'use');
  s.setAttribute('class', 'ic' + (cls ? ' ' + cls : '')); s.setAttribute('aria-hidden', 'true');
  u.setAttribute('href', '#i-' + name); s.append(u);
  return s;
}
const withIcon = (name, label, cls) => [icon(name, cls || 'sm'), label];

function baseUrl(s) {
  let [host, port] = s.addr.replace(/^\[|\]$/g, '').split(/:(?=[^:]*$)/);
  if (s.addr.startsWith('[')) { const m = /^\[(.*)\]:(\d+)$/.exec(s.addr); host = m[1]; port = m[2]; }
  if (host === '0.0.0.0' || host === '::') host = location.hostname;
  return `${s.tls ? 'https' : 'http'}://${host.includes(':') ? '[' + host + ']' : host}:${port}`;
}
function snippet(label, code) {
  return h('div', { class: 'snip-code' }, label ? h('div', { class: 'mut l', text: label }) : null, h('pre', { text: code }), h('button', { class: 'act', type: 'button', onclick: () => copy(code) }, ...withIcon('copy', t('key.copy'))));
}
function howTo(s) {
  const base = baseUrl(s), c = s.config;
  const key = c.root_key && (c.auth === 'key' || c.auth === 'both') ? c.root_key : (c.auth === 'accounts' ? t('svc.placeholder') : null);
  const auth = key ? `-H 'Authorization: Bearer ${key}' ` : '';
  const mcpUrl = base + '/mcp';
  const cfg = { mcpServers: { seesea: { type: 'http', url: mcpUrl, ...(key ? { headers: { Authorization: 'Bearer ' + key } } : {}) } } };
  const box = h('div', { class: 'snips' }, h('h4', { text: t('svc.how') }));
  if (s.id === 'api') box.append(snippet('curl', `curl ${auth}'${base}/v1/search?q=rust+async+runtime&mode=fast'`));
  box.append(snippet('Claude Code', `claude mcp add --transport http seesea ${mcpUrl}${key ? ` --header "Authorization: Bearer ${key}"` : ''}`));
  box.append(snippet('mcp.json (Cursor / Claude Desktop / …)', JSON.stringify(cfg, null, 2)));
  if (s.id === 'mcp') box.append(snippet(t('svc.stdio'), JSON.stringify({ mcpServers: { seesea: { command: 'seesea', args: ['mcp'] } } }, null, 2)));
  return box;
}

const SVC_UI = { api: { tab: 'overview', shown: false }, mcp: { tab: 'overview', shown: false } };
const stopTimer = () => { if (S.timer) { clearInterval(S.timer); S.timer = null; } };

async function loadServices() {
  const out = $('#svc-out');
  try { renderServices((await api('/desk/api/state')).services); } catch (e) { out.replaceChildren(errBox(e, loadServices)); }
  stopTimer();
  // Counters and the log move while the page is open; forms do not, so only those parts are redrawn.
  S.timer = setInterval(async () => {
    if (S.view !== 'services' || document.hidden) return;
    try {
      const st = (await api('/desk/api/state')).services;
      for (const s of st) { const cur = (S.svcs || []).find((x) => x.id === s.id); if (cur && (cur.running !== s.running || cur.addr !== s.addr)) return renderServices(st); }
      S.svcs = st; for (const s of st) paintStats(s);
      const log = $('.logpane'); if (log) refreshLog(log.dataset.id);
    } catch { /* the error box appears on the next action */ }
  }, 3000);
}
function renderServices(list) { S.svcs = list; $('#svc-out').replaceChildren(...list.map(svcCard)); }

const fmtUp = (n) => (n >= 3600 ? `${Math.floor(n / 3600)}h ${Math.floor(n % 3600 / 60)}m` : n >= 60 ? `${Math.floor(n / 60)}m ${n % 60}s` : `${n}s`);
function statCell(id, key, label, value, cls) { return h('div', { class: 'stat' + (cls ? ' ' + cls : ''), 'data-k': `${id}.${key}` }, h('b', { text: value }), h('span', { text: label })); }
function statsRow(s) {
  const st = s.stats || {};
  return [
    statCell(s.id, 'total', t('sv.req'), st.total ?? 0),
    statCell(s.id, 'ok', t('sv.ok'), st.ok ?? 0),
    statCell(s.id, 'err', t('sv.err'), (st.client_error || 0) + (st.server_error || 0), (st.client_error || 0) + (st.server_error || 0) ? 'bad' : ''),
    statCell(s.id, 'up', t('sv.up'), fmtUp(st.up_secs || 0)),
    statCell(s.id, 'auth', t('f.auth'), t('auth.' + s.config.auth).replace(/（.*|\(.*/, ''), 'txt'),
  ];
}
function paintStats(s) {
  const st = s.stats; if (!st) return;
  const set = (k, v, bad) => { const c = $(`.stat[data-k="${s.id}.${k}"]`); if (c) { const b = $('b', c); if (b.textContent !== String(v) && k !== 'up') replay(b, 'bump', 600); b.textContent = v; c.classList.toggle('bad', !!bad); } };
  const e = (st.client_error || 0) + (st.server_error || 0);
  set('total', st.total); set('ok', st.ok); set('err', e, e); set('up', fmtUp(st.up_secs));
}

function svcCard(s) {
  const ui = SVC_UI[s.id], d = structuredClone(s.config);
  const redraw = () => card.replaceWith(svcCard({ ...s, config: d }));
  const call = async (action, body, done) => {
    try { const r = await api(`/desk/api/services/${s.id}/${action}`, { method: 'POST', body }); renderServices(r.services); toast(done); } catch (e) { fail(e); }
  };
  const apply = async (action, done) => {
    try { await api(`/desk/api/services/${s.id}/config`, { method: 'POST', body: d }); } catch (e) { return fail(e); }
    call(action, undefined, done);
  };
  const tabBtn = (id, label, ic, needRun) => h('button', { type: 'button', class: ui.tab === id ? 'on' : '', disabled: needRun && !s.running, onclick: () => { ui.tab = id; redraw(); } }, icon(ic, 'sm'), label);
  const body = ui.tab === 'console' && s.running ? consolePane(s, d) : ui.tab === 'log' && s.running ? logPane(s) : overviewPane(s, d, redraw, apply);
  const card = h('section', { class: 'svc' + (s.running ? ' run' : '') },
    h('div', { class: 'svc-h' },
      h('span', { class: 'lamp ' + (s.running ? 'ok' : '') }),
      h('h2', {}, h('span', { text: t('svc.' + s.id) })),
      s.running ? h('span', { class: 'addr' }, baseUrl(s), h('button', { class: 'act', type: 'button', title: t('key.copy'), onclick: () => copy(baseUrl(s)) }, icon('copy', 'sm'))) : h('span', { class: 'pill', text: t('svc.stopped') }),
      h('span', { class: 'sp' }),
      s.running
        ? [h('button', { class: 'quiet', type: 'button', onclick: () => apply('restart', t('svc.started')) }, ...withIcon('refresh', t('svc.restart'))), h('button', { class: 'quiet danger', type: 'button', onclick: () => call('stop', undefined, t('svc.stopped.t')) }, ...withIcon('stop', t('svc.stop')))]
        : [h('button', { class: 'go', type: 'button', onclick: () => apply('start', t('svc.started')) }, icon('play', 'sm'), t('svc.start'))],
      h('p', { text: t('svc.' + s.id + '.d') })),
    s.problem ? h('div', { class: 'bad-msg', text: s.problem }) : null,
    s.running ? h('div', { class: 'stats' }, statsRow(s)) : null,
    h('div', { class: 'tabs' }, tabBtn('overview', t('sv.overview'), 'services'), tabBtn('console', t('sv.console'), 'send', true), tabBtn('log', t('sv.log'), 'status', true)),
    h('div', { class: 'tabbody' }, body));
  return card;
}

function overviewPane(s, d, redraw, apply) {
  const f = (label, input, cls) => h('label', { class: cls || '' }, h('span', { text: label }), input);
  const text = (k, ph) => h('input', { value: d[k] || '', placeholder: ph || '', spellcheck: 'false', oninput: (e) => { d[k] = e.target.value || null; } });
  const check = (k, label) => h('label', { class: 'check' }, h('input', { type: 'checkbox', checked: d[k], onchange: (e) => { d[k] = e.target.checked; } }), label);
  const authSel = h('select', { onchange: (e) => { d.auth = e.target.value; if ((d.auth === 'key' || d.auth === 'both') === false) d.root_key = null; redraw(); } },
    ['open', 'key', 'accounts', 'both'].map((v) => h('option', { value: v, selected: d.auth === v, text: t('auth.' + v) })));
  const ui = SVC_UI[s.id];
  const call = async (action, done) => { try { const r = await api(`/desk/api/services/${s.id}/${action}`, { method: 'POST', body: {} }); renderServices(r.services); toast(done); } catch (e) { fail(e); } };
  const keyRow = (d.auth === 'key' || d.auth === 'both') ? f(t('f.key'), h('div', { class: 'keybox' },
    h('code', { text: d.root_key ? (ui.shown ? d.root_key : '•'.repeat(24)) : t('svc.auto') }),
    d.root_key ? h('button', { class: 'act', type: 'button', onclick: () => { ui.shown = !ui.shown; redraw(); } }, ...withIcon('eye', ui.shown ? t('key.hide') : t('key.show'))) : null,
    d.root_key ? h('button', { class: 'act', type: 'button', onclick: () => copy(d.root_key) }, ...withIcon('copy', t('key.copy'))) : null,
    s.config.root_key ? h('button', { class: 'act', type: 'button', onclick: async () => { if (await confirmBox(t('key.rotate'), t('key.rotate.q'), t('key.rotate'))) call('rotate', t('key.rotate')); } }, ...withIcon('refresh', t('key.rotate'))) : null), 'full') : null;
  const local = /^(127\.|\[::1\]|localhost)/.test((d.addr || '').trim());
  const warn = !local && (d.auth === 'open') ? t('svc.warn.open') : (!local && !d.tls_cert && !d.behind_tls_proxy ? t('svc.warn.net') : null);
  return h('div', {},
    h('div', { class: 'form' },
      f(t('f.addr'), text('addr', '127.0.0.1:8080')),
      f(t('f.auth'), authSel),
      keyRow,
      f(t('f.cert'), text('tls_cert', '/etc/seesea/cert.pem')),
      f(t('f.tlskey'), text('tls_key', '/etc/seesea/key.pem')),
      f(t('f.conc'), h('input', { type: 'number', min: 1, max: 4096, value: d.max_concurrent, oninput: (e) => { d.max_concurrent = +e.target.value || 64; } })),
      check('behind_tls_proxy', t('f.proxy')), check('allow_open', t('f.open')), check('autostart', t('f.auto')),
      warn ? h('div', { class: 'warn-msg full', text: warn }) : null,
      h('div', { class: 'full row' }, h('button', { class: 'quiet', type: 'button', onclick: async () => { try { const r = await api(`/desk/api/services/${s.id}/config`, { method: 'POST', body: d }); renderServices(r.services); toast(t('svc.saved')); } catch (e) { fail(e); } } }, ...withIcon('check', t('svc.save'))))),
    s.running ? howTo(s) : null);
}

/* the console: send one request to the running service from here */
const PRESETS = {
  api: [
    ['search GET', 'GET', '/v1/search?q=rust+async+runtime&mode=fast', ''],
    ['search POST', 'POST', '/v1/search', '{\n  "query": "rust async runtime",\n  "mode": "fast",\n  "limit": 5\n}'],
    ['extract', 'POST', '/v1/extract', '{\n  "url": "https://example.com"\n}'],
    ['sources', 'GET', '/v1/sources', ''], ['stats', 'GET', '/v1/stats', ''], ['index', 'GET', '/v1/index', ''],
  ],
  mcp: [],
};
const MCP_PRESETS = [
  ['mcp tools/list', 'POST', '/mcp', '{\n  "jsonrpc": "2.0",\n  "id": 1,\n  "method": "tools/list"\n}'],
  ['mcp search', 'POST', '/mcp', '{\n  "jsonrpc": "2.0",\n  "id": 2,\n  "method": "tools/call",\n  "params": { "name": "search", "arguments": { "query": "rust async runtime", "mode": "fast" } }\n}'],
];
function consolePane(s, d) {
  const ui = SVC_UI[s.id];
  ui.req = ui.req || { auth: s.config.auth === 'open' ? 'none' : s.config.root_key && (s.config.auth === 'key' || s.config.auth === 'both') ? 'shared' : 'key', key: '' };
  const r = ui.req;
  const presets = [...PRESETS[s.id], ...MCP_PRESETS, ['health', 'GET', '/health', '']];
  if (!r.path) Object.assign(r, { method: presets[0][1], path: presets[0][2], body: presets[0][3] });
  const method = h('select', { onchange: (e) => { r.method = e.target.value; bodyBox.hidden = r.method === 'GET'; } }, ['GET', 'POST'].map((m) => h('option', { value: m, selected: r.method === m, text: m })));
  const path = h('input', { value: r.path, spellcheck: 'false', oninput: (e) => { r.path = e.target.value; } });
  const bodyBox = h('textarea', { spellcheck: 'false', hidden: r.method === 'GET', oninput: (e) => { r.body = e.target.value; } });
  bodyBox.value = r.body;
  const keyIn = h('input', { type: 'password', placeholder: 'ss_… / root key', value: r.key, autocomplete: 'off', hidden: r.auth !== 'key', oninput: (e) => { r.key = e.target.value; } });
  const authOpts = [['none', t('cs.none')]];
  if (s.config.root_key && (s.config.auth === 'key' || s.config.auth === 'both')) authOpts.push(['shared', t('cs.shared')]);
  authOpts.push(['key', t('cs.custom')]);
  if (!authOpts.some(([v]) => v === r.auth)) r.auth = 'none';
  const authSel = h('select', { onchange: (e) => { r.auth = e.target.value; keyIn.hidden = r.auth !== 'key'; } }, authOpts.map(([v, l]) => h('option', { value: v, selected: r.auth === v, text: l })));
  const out = h('div', {});
  const send = h('button', { class: 'go', type: 'button' }, icon('send', 'sm'), t('cs.send'));
  send.addEventListener('click', async () => {
    send.disabled = true; out.replaceChildren(h('div', { class: 'wait' }, h('div')));
    try {
      const x = await api(`/desk/api/services/${s.id}/try`, { method: 'POST', body: { method: r.method, path: r.path, body: r.method === 'POST' ? r.body : undefined, auth: r.auth, key: r.auth === 'key' ? r.key : undefined } });
      let text = x.body; try { text = JSON.stringify(JSON.parse(x.body), null, 2); } catch { /* not JSON */ }
      out.replaceChildren(h('div', { class: 'resp' },
        h('div', { class: 'resp-h' }, h('span', { class: 'st s' + Math.floor(x.status / 100), text: x.status }), h('span', { text: `${x.ms} ms` }), ...(x.headers || []).filter(([k]) => k !== 'content-type').map(([k, v]) => h('span', { class: 'mut', text: `${k}: ${v}` })), h('span', { class: 'sp' }),
          h('button', { class: 'act', type: 'button', onclick: () => copy(text) }, ...withIcon('copy', t('key.copy')))),
        h('pre', { text: text + (x.truncated ? '\n…' : '') })));
    } catch (e) { out.replaceChildren(errBox(e)); } finally { send.disabled = false; }
  });
  const chips = presets.map(([label, m, p, b]) => h('button', { class: 'chipbtn' + (r.path === p && r.method === m && r.body === b ? ' on' : ''), type: 'button', text: label, onclick: () => { Object.assign(r, { method: m, path: p, body: b }); method.value = m; path.value = p; bodyBox.value = b; bodyBox.hidden = m === 'GET'; $$('.chipbtn', chips).forEach((c) => c.classList.toggle('on', c.textContent === label)); } }));
  const chipRow = h('div', { class: 'pre' }, chips);
  return h('div', { class: 'console' }, chipRow,
    h('div', { class: 'req' }, method, path),
    bodyBox,
    h('div', { class: 'row' }, h('label', { class: 'fld' }, h('span', { text: t('cs.auth') }), authSel), keyIn, h('span', { class: 'sp' }), send),
    out);
}

/* the log: what the service answered lately, without the query text or the key */
function logPane(s) {
  const box = h('div', { class: 'logpane', 'data-id': s.id }, h('div', { class: 'mut l', text: t('st.probe') }));
  refreshLog(s.id);
  return box;
}
async function refreshLog(id) {
  const box = $('.logpane'); if (!box || box.dataset.id !== id) return;
  try {
    const r = await api(`/desk/api/services/${id}/log`);
    const rows = r.recent || [];
    const time = (ts) => new Date(ts * 1000).toLocaleTimeString(S.lang === 'zh' ? 'zh-CN' : 'en-GB', { hour12: false });
    box.replaceChildren(
      h('div', { class: 'logbar' }, h('span', { text: `${t('sv.req')} ${r.summary.total}` }), h('span', { text: `2xx ${r.summary.ok}` }), h('span', { text: `4xx ${r.summary.client_error}` }), h('span', { text: `5xx ${r.summary.server_error}` }), h('span', { class: 'sp' }), h('span', { text: t('lg.auto') })),
      rows.length ? h('table', { class: 'logt' }, h('tr', {}, h('th', { text: t('lg.time') }), h('th', { text: t('lg.req') }), h('th', { text: t('lg.status') }), h('th', { class: 'r', text: 'ms' }), h('th', { text: t('lg.who') })),
        rows.map((e) => h('tr', {}, h('td', { text: time(e.ts) }), h('td', { class: 'clip', text: `${e.method} ${e.path}` }), h('td', {}, h('span', { class: 'st s' + Math.floor(e.status / 100), text: e.status })), h('td', { class: 'r', text: e.ms }), h('td', { class: 'who', text: e.who || '—' })))) : h('div', { class: 'mut l', text: t('lg.none') }));
  } catch (e) { box.replaceChildren(errBox(e, () => refreshLog(id))); }
}

/* ---------- accounts (workbench only) ---------- */
let secretBanner = null;
const lim = (n) => (!n || n >= 1e8 ? '∞' : n.toLocaleString());
function spark(usage, today) {
  const byDay = Object.fromEntries((usage || []).map((u) => [u.day, u.credits]));
  const vals = Array.from({ length: 7 }, (_, i) => byDay[today - 6 + i] || 0), max = Math.max(...vals, 1);
  const s = document.createElementNS(SVGNS, 'svg');
  s.setAttribute('class', 'spark'); s.setAttribute('viewBox', '0 0 84 26'); s.setAttribute('role', 'img'); s.setAttribute('aria-label', vals.join(' '));
  vals.forEach((v, i) => {
    const r = document.createElementNS(SVGNS, 'rect'), hgt = Math.max(2, (v / max) * 24);
    r.setAttribute('x', i * 12 + 1); r.setAttribute('width', 10); r.setAttribute('y', 25 - hgt); r.setAttribute('height', hgt);
    if (i === 6) r.setAttribute('class', 'today');
    const tt = document.createElementNS(SVGNS, 'title'); tt.textContent = String(v); r.append(tt); s.append(r);
  });
  return s;
}

async function loadAccounts() {
  const out = $('#acc-out');
  try {
    const r = await api('/desk/api/users');
    const table = r.plan_table || {};
    const name = h('input', { placeholder: t('acc.name'), maxlength: 64 });
    const plan = h('select', {}, (r.plans || []).sort().map((p) => h('option', { value: p, selected: p === 'default', text: p })));
    const add = h('form', { class: 'inline-form', onsubmit: async (e) => { e.preventDefault(); try { await api('/desk/api/users', { method: 'POST', body: { name: name.value, plan: plan.value } }); loadAccounts(); } catch (x) { fail(x); } } },
      name, plan, h('button', { class: 'go' }, icon('plus', 'sm'), t('acc.add')));
    const when = (n) => (n ? ago(n) : t('acc.never'));
    const kids = [];
    if (secretBanner) { const k = secretBanner; secretBanner = null; kids.push(h('div', { class: 'panel secret' }, h('div', { class: 'warn-msg', text: t('acc.secret') }), h('div', { class: 'keybox', style: 'margin-top:8px' }, h('code', { text: k }), h('button', { class: 'act', type: 'button', onclick: () => copy(k) }, ...withIcon('copy', t('key.copy')))))); }
    kids.push(add);

    const uses = {}; for (const u of r.users || []) uses[u.plan] = (uses[u.plan] || 0) + 1;
    kids.push(h('div', { class: 'plans' }, h('div', { class: 'ph' }, h('h2', {}, h('span', { text: t('pl.title') }), h('em', { text: S.lang === 'zh' ? 'PLANS' : '套餐' }))),
      h('table', {}, h('tr', {}, h('th', { text: t('acc.plan') }), h('th', { class: 'r', text: t('pl.min') }), h('th', { class: 'r', text: t('pl.burst') }), h('th', { class: 'r', text: t('pl.day') }), h('th', { class: 'r', text: t('pl.conc') }), h('th', { text: t('pl.deep') }), h('th', { class: 'r', text: t('pl.users') })),
        Object.entries(table).sort().map(([n, p]) => h('tr', {}, h('td', {}, h('b', { text: n })), h('td', { class: 'r num', text: lim(p.per_minute) }), h('td', { class: 'r num', text: lim(p.burst) }), h('td', { class: 'r num', text: lim(p.per_day) }), h('td', { class: 'r num', text: lim(p.max_concurrent) }), h('td', {}, p.deep ? icon('check', 'sm') : '—'), h('td', { class: 'r num', text: uses[n] || 0 }))))));

    kids.push(h('div', { class: 'ph' }, h('h2', {}, h('span', { text: t('us.title') }), h('em', { text: S.lang === 'zh' ? 'USERS' : '用户' }))));
    const users = (r.users || []).map((u) => {
      const p = table[u.plan] || {}, cap = p.per_day, hot = cap && u.used_today / cap > 0.85;
      const planSel = h('select', { onchange: async (e) => { try { await api(`/desk/api/users/${encodeURIComponent(u.name)}/plan`, { method: 'POST', body: { plan: e.target.value } }); toast(t('svc.saved')); loadAccounts(); } catch (x) { fail(x); } } },
        (r.plans || []).sort().map((n) => h('option', { value: n, selected: n === u.plan, text: n })));
      const live = u.keys.filter((k) => !k.revoked).length;
      return h('div', { class: 'user' + (u.disabled ? ' off' : '') },
        h('div', { class: 'user-h' },
          h('div', { class: 'nm' }, u.name, u.disabled ? h('span', { class: 'pill off', text: t('acc.off') }) : h('span', { class: 'pill ok', text: `${live} ${t('us.keys')}` }), planSel),
          h('div', { class: 'quota' }, h('div', { text: `${t('acc.today')}  ${u.used_today.toLocaleString()} / ${lim(cap)}` }), h('div', { class: 't' }, h('i', { class: hot ? 'hot' : '', style: `width:${cap ? Math.min(100, (u.used_today / cap) * 100) : 0}%` }))),
          spark(u.usage, u.today),
          h('div', { class: 'row end' },
            h('button', { class: 'quiet', type: 'button', onclick: async () => { const o = await askNewKey(u.name); if (!o) return; try { const k = await api(`/desk/api/users/${encodeURIComponent(u.name)}/keys`, { method: 'POST', body: o }); secretBanner = k.key; loadAccounts(); } catch (x) { fail(x); } } }, ...withIcon('plus', t('acc.newkey'))),
            h('button', { class: 'quiet' + (u.disabled ? '' : ' danger'), type: 'button', onclick: async () => {
              if (!u.disabled && !(await confirmBox(t('acc.disable'), t('acc.disable.q', { n: u.name }), t('acc.disable')))) return;
              try { await api(`/desk/api/users/${encodeURIComponent(u.name)}/disabled`, { method: 'POST', body: { disabled: !u.disabled } }); loadAccounts(); } catch (x) { fail(x); } } }, u.disabled ? t('acc.enable') : t('acc.disable')))),
        u.keys.length ? h('div', { class: 'keys' }, h('table', {}, h('tr', {}, h('th', { text: 'ID' }), h('th', { text: t('nk.name') }), h('th', { text: t('acc.used') }), h('th', { text: t('acc.expires') }), h('th')),
          u.keys.map((k) => h('tr', { class: k.revoked ? 'gone' : '' }, h('td', { class: 'id', text: `${k.id}_…` }), h('td', { text: k.name }), h('td', { text: when(k.last_used) }), h('td', { text: k.expires ? ago(k.expires) : t('nk.never') }),
            h('td', { class: 'r' }, k.revoked ? h('span', { class: 'pill off', text: t('acc.revoked') }) : h('button', { class: 'act danger', type: 'button', onclick: async () => {
              if (!(await confirmBox(t('acc.revoke'), t('acc.revoke.q', { id: k.id }), t('acc.revoke')))) return;
              try { await api(`/desk/api/keys/${k.id}/revoke`, { method: 'POST', body: {} }); loadAccounts(); } catch (x) { fail(x); } } }, t('acc.revoke'))))))) : h('div', { class: 'keys' }, h('div', { class: 'mut l', text: t('acc.nokeys') })));
    });
    kids.push(users.length ? h('div', { class: 'users' }, users) : h('div', { class: 'empty', text: t('acc.none') }));
    out.replaceChildren(...kids);
  } catch (e) { out.replaceChildren(errBox(e, loadAccounts)); }
}

/* small dialogs */
function askNewKey(user) {
  const dlg = $('#dlg-newkey');
  $('#nk-user').textContent = user; $('#nk-name').value = ''; $('#nk-exp').value = '';
  return new Promise((resolve) => {
    dlg.addEventListener('close', () => resolve(dlg.returnValue === 'ok' ? { name: $('#nk-name').value.trim() || 'key', expires_days: $('#nk-exp').value ? +$('#nk-exp').value : undefined } : null), { once: true });
    dlg.returnValue = ''; dlg.showModal(); $('#nk-name').focus();
  });
}
function confirmBox(title, msg, okLabel) {
  const dlg = $('#dlg-confirm');
  $('#cf-title').textContent = title; $('#cf-msg').textContent = msg; $('#cf-ok').textContent = okLabel;
  return new Promise((resolve) => {
    dlg.addEventListener('close', () => resolve(dlg.returnValue === 'ok'), { once: true });
    dlg.returnValue = ''; dlg.showModal();
  });
}

/* ---------- review (workbench only) ---------- */
function renderReview(r) {
  const out = $('#rv-out'), rep = r.report;
  if (!rep.searches && !(rep.candidates || []).length) { out.replaceChildren(h('div', { class: 'empty' }, h('div', { text: t('rv.none') }))); return; }
  const tile = (n, label) => h('div', { class: 'tile' }, h('b', { text: n }), h('span', { text: label }));
  const errs = Object.values(rep.errors || {}).reduce((a, b) => a + b, 0);
  const kids = [h('div', { class: 'tiles' },
    tile(rep.searches, t('rv.searches')), tile(rep.cached, t('rv.cached')), tile(`${rep.took_ms_p50} / ${rep.took_ms_p95}`, t('rv.latency')), tile(errs, t('rv.errors')), tile(`${rep.feedback_good} / ${rep.feedback_bad}`, t('rv.fb')))];

  const srcRows = Object.entries(rep.sources || {});
  const unread = rep.unreadable_hosts || [];
  const stand = (x, bad) => h('div', { class: 'stand' + (bad ? ' bad' : ''), title: t('rv.obs', { n: x.observations.toFixed(1) }) }, h('span', { class: 'clip', text: x.name }), h('div', { class: 't' }, h('i', { style: `width:${Math.max(0, Math.min(1, x.value)) * 100}%` })), h('span', { class: 'mut', text: x.value.toFixed(2) }));
  const L = r.learn || {};
  kids.push(h('div', { class: 'grid2' },
    h('div', { class: 'panel' }, h('div', { class: 'ph' }, h('h2', { text: t('rv.src') })), srcRows.length ? h('table', {}, srcRows.map(([n, v]) => h('tr', {}, h('td', { text: n }), h('td', { class: 'r', text: `${v.answered} ${t('rv.answered')}` }), h('td', { class: 'r' }, v.failed ? h('span', { class: 'chip2', text: `${v.failed} ${t('rv.failed')}: ` + Object.entries(v.by_kind || {}).map(([k, c]) => `${k} ${c}`).join(', ') }) : null)))) : h('div', { class: 'mut', text: t('st.none') })),
    h('div', { class: 'panel' }, h('div', { class: 'ph' }, h('h2', { text: t('rv.unread') })), unread.length ? h('table', {}, unread.slice(0, 12).map(([host, n, why]) => h('tr', {}, h('td', { text: host }), h('td', { class: 'r', text: n }), h('td', { class: 'mut l', text: why })))) : h('div', { class: 'mut', text: t('st.none') })),
    h('div', { class: 'panel' }, h('div', { class: 'ph' }, h('h2', { text: t('rv.learn') }), h('span', { class: 'mut', text: t('rv.learned', { o: (L.observations || 0).toFixed(0), h: L.hosts || 0 }) })),
      (L.best_hosts || []).length ? [h('div', { class: 'sub', text: t('rv.best') }), L.best_hosts.map((x) => stand(x, false)), h('div', { class: 'sub', text: t('rv.worst') }), L.worst_hosts.map((x) => stand(x, true))] : h('div', { class: 'mut', text: t('st.none') }))));

  const sus = rep.suspects || [];
  kids.push(h('div', { class: 'panel gap' }, h('div', { class: 'ph' }, h('h2', { text: t('rv.sus') }), h('span', { class: 'mut', text: sus.length })),
    sus.length ? sus.slice(-30).reverse().map((x) => h('div', { class: 'sus' }, h('q', { text: x.query }), h('button', { class: 'act', type: 'button', text: t('rv.again'), onclick: () => { S.mode = x.mode || 'standard'; $$('#mode button').forEach((b) => b.classList.toggle('on', b.dataset.v === S.mode)); $('#q').value = x.query; location.hash = '#/search'; $('#search-form').requestSubmit(); } }),
      h('div', { class: 'chips' }, (x.why || []).map((w) => h('span', { class: 'chip2', text: w })), h('span', { class: 'mut', text: ago(x.ts) })))) : h('div', { class: 'mut', text: t('rv.nosus') })));

  const prog = h('span', { class: 'mut' });
  const setProg = (j) => { prog.textContent = t('rv.progress', { n: j.graded ?? j.judgments.graded, q: j.queries ?? j.judgments.queries }); };
  setProg(r);
  const file = r.judgments.file;
  kids.push(h('div', { class: 'ph' }, h('h2', { text: t('rv.grade') }), prog));
  kids.push(h('p', { class: 'mut l scale', text: t('rv.scale') }));
  const groups = rep.candidates || [];
  if (!groups.length) kids.push(h('div', { class: 'mut', text: t('rv.nocand') }));
  for (const g of groups) {
    kids.push(h('div', { class: 'group' }, h('h3', {}, h('span', { text: g.query }), h('small', { text: `${g.candidates.length}` })), g.candidates.map((c) => {
      const btns = [0, 1, 2, 3].map((n) => h('button', { type: 'button', 'data-g': n, class: c.grade === n ? 'on' : '', text: n, onclick: async () => {
        const next = c.grade === n ? null : n;
        try { setProg(await api('/desk/api/grade', { method: 'POST', body: { query: g.query, url: c.url, grade: next } })); c.grade = next; btns.forEach((b, i) => b.classList.toggle('on', c.grade === i)); } catch (e) { fail(e); }
      } }));
      const href = safeHref(c.url), host = hostOf(c.url);
      return h('div', { class: 'gr' }, h('span', { class: 'n', text: c.rank }), h('div', { class: 'ti' }, href ? h('a', { href, target: '_blank', rel: 'noopener noreferrer', text: c.title || c.url }) : (c.title || c.url), h('small', { text: host })), h('div', { class: 'grades' }, btns));
    })));
  }
  kids.push(h('div', { class: 'snips boxed' }, h('h4', { text: t('rv.file') }), snippet('', file), h('h4', { text: t('rv.run') }), snippet('', `seesea eval ${file}`)));
  out.replaceChildren(...kids);
}
async function loadReview() {
  const out = $('#rv-out');
  try { renderReview(await api('/desk/api/review?since=' + $('#rv-since').value)); } catch (e) { out.replaceChildren(errBox(e, loadReview)); }
}

/* ---------- shell ---------- */
const VIEWS = { search: () => $('#q').focus(), read: () => $('#r-url').focus(), review: loadReview, status: loadStatus, services: loadServices, accounts: loadAccounts };
function route() {
  let v = (location.hash.match(/^#\/(\w+)/) || [])[1] || 'search';
  const deskOnly = v === 'services' || v === 'accounts' || v === 'review';
  if (location.pathname !== '/') return errorPage(404, 'err.nf', t('err.nf.d', { p: location.pathname }));
  if (!VIEWS[v]) return errorPage(404, 'err.nf', t('err.nf.d', { p: location.hash || '#' }));
  if (deskOnly && !S.desk) return errorPage(404, 'err.na', t('err.na.d'));
  const changed = S.view !== v;
  for (const el of $$('.view')) el.hidden = el.id !== 'v-' + v;
  for (const a of $$('#nav a')) a.classList.toggle('on', a.dataset.view === v);
  moveInd();
  if (changed) { replay($('#v-' + v), 'enter', 1000); replay($('#crumb'), 'swap', 400); }
  S.view = v;
  stopTimer();
  $('#crumb').textContent = t('nav.' + v);
  VIEWS[v]();
}
function errorPage(code, titleKey, msg) {
  for (const el of $$('.view')) el.hidden = el.id !== 'v-error';
  for (const a of $$('#nav a')) a.classList.remove('on');
  S.view = 'error';
  replay($('#v-error'), 'enter', 1000); moveInd();
  stopTimer();
  $('#crumb').textContent = code;
  $('#err-code').textContent = code;
  $('#err-title').textContent = t(titleKey);
  $('#err-msg').textContent = msg;
  // Off the root path a bare #/x link would stay on the missing page.
  for (const a of $$('#nav a, .brand')) a.setAttribute('href', '/' + a.getAttribute('href').replace(/^\/?/, ''));
  $('#err-home').href = '/#/search';
  $('#err-retry').onclick = () => location.reload();
}
function updateChip() {
  const chip = $('#where');
  $('#ver').textContent = S.health ? `v${S.health.version}` : '';
  $('#lang-lbl').textContent = S.lang === 'zh' ? 'EN' : '中';
  if (!S.health) { chip.className = 'chip down'; $('span', chip).textContent = t('where.down'); return; }
  chip.className = 'chip up';
  $('span', chip).textContent = S.desk ? t('where.desk') : location.host;
}

/* palette */
function commands() {
  const go = (v) => () => { location.hash = '#/' + v; };
  const list = [['search', 'nav.search'], ['read', 'nav.read'], ['status', 'nav.status']];
  if (S.desk) list.push(['review', 'nav.review'], ['services', 'nav.services'], ['accounts', 'nav.accounts']);
  return [
    ...list.map(([v, k]) => ({ label: `${t('p.go')} · ${t(k)}`, hint: '#/' + v, run: go(v) })),
    { label: t('p.focus'), hint: '/', run: () => { location.hash = '#/search'; $('#q').focus(); } },
    { label: t('p.theme'), run: toggleTheme },
    { label: t('p.lang'), run: toggleLang },
    { label: t('p.key'), run: async () => { const k = await askKey(); if (k) session.set('ss_key', k); } },
  ];
}
function openPalette() {
  const dlg = $('#dlg-palette'), input = $('#pal-in'), ul = $('#pal-list');
  let items = [], on = 0;
  const draw = () => {
    const q = input.value.trim().toLowerCase();
    items = commands().filter((c) => c.label.toLowerCase().includes(q));
    on = Math.min(on, Math.max(0, items.length - 1));
    ul.replaceChildren(...items.map((c, i) => h('li', { class: i === on ? 'on' : '', onclick: () => run(i) }, c.label, c.hint ? h('small', { text: c.hint }) : null)));
  };
  const run = (i) => { dlg.close(); items[i] && items[i].run(); };
  input.value = ''; input.placeholder = t('p.ph'); on = 0; draw();
  input.oninput = () => { on = 0; draw(); };
  input.onkeydown = (e) => {
    if (e.key === 'ArrowDown') { on = Math.min(items.length - 1, on + 1); draw(); e.preventDefault(); }
    else if (e.key === 'ArrowUp') { on = Math.max(0, on - 1); draw(); e.preventDefault(); }
    else if (e.key === 'Enter') { run(on); e.preventDefault(); }
  };
  dlg.showModal(); input.focus();
}
function toggleLang() {
  S.lang = S.lang === 'zh' ? 'en' : 'zh'; store.set('ss_lang', S.lang); applyLang(); replay($('main'), 'langfade', 400); setTimeout(moveInd, 0);
  if (S.last) renderSearch(...S.last); else { $('#search-out').replaceChildren(); emptyState(); }
  if (S.lastRead) renderRead(S.lastRead);
  route();
}

// Nothing is shown before the first search: the box and its options are the page.
function emptyState() {}

async function boot() {
  S.lang = store.get('ss_lang') || ((navigator.language || 'en').toLowerCase().startsWith('zh') ? 'zh' : 'en');
  applyTheme();
  try { S.health = await api('/health', { noAuth: true }); } catch { S.health = null; }
  try { const st = await api('/desk/api/state', { noAuth: true }); S.desk = !!(st && st.services); } catch { S.desk = false; }
  for (const el of $$('.desk-only')) el.hidden = !S.desk;
  applyLang();
  try { const o = JSON.parse(store.get('ss_opts') || '{}'); if (o.mode) S.mode = o.mode; for (const [id, k] of [['o-time', 'time'], ['o-topic', 'topic'], ['o-limit', 'limit'], ['o-inc', 'inc'], ['o-exc', 'exc']]) if (o[k] != null) $('#' + id).value = o[k]; } catch { /* ignore */ }
  $$('#mode button').forEach((b) => b.classList.toggle('on', b.dataset.v === S.mode));
  emptyState();
  route();
  if (document.fonts) document.fonts.ready.then(moveInd);
  setInterval(async () => {
    try { S.health = await api('/health', { noAuth: true }); } catch { S.health = null; }
    updateChip();
  }, 15000);
}

$('#search-form').addEventListener('submit', (e) => {
  e.preventDefault();
  store.set('ss_opts', JSON.stringify({ mode: S.mode, time: $('#o-time').value, topic: $('#o-topic').value, limit: $('#o-limit').value, inc: $('#o-inc').value, exc: $('#o-exc').value }));
  doSearch();
});
$('#read-form').addEventListener('submit', (e) => { e.preventDefault(); doRead(); });
$('#mode').addEventListener('click', (e) => {
  const b = e.target.closest('button'); if (!b) return;
  S.mode = b.dataset.v;
  $$('#mode button').forEach((x) => x.classList.toggle('on', x === b));
});
$('#btn-check').addEventListener('click', probe);
$('#rv-reload').addEventListener('click', loadReview);
$('#rv-since').addEventListener('change', loadReview);
$('#btn-theme').addEventListener('click', toggleTheme);
$('#btn-lang').addEventListener('click', toggleLang);
$('#btn-palette').addEventListener('click', openPalette);
$('#btn-key').addEventListener('click', async () => { const k = await askKey(); if (k) session.set('ss_key', k); });
window.addEventListener('hashchange', route);
document.addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); openPalette(); return; }
  const typing = /^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement.tagName) || document.querySelector('dialog[open]');
  if (typing) { if (e.key === 'Escape') document.activeElement.blur(); return; }
  if (e.key === '/') { e.preventDefault(); location.hash = '#/search'; $('#q').focus(); return; }
  if (S.view !== 'search') return;
  if (e.key === 'j') select(S.sel + 1);
  else if (e.key === 'k') select(S.sel - 1);
  else if (e.key === 'd' && S.sel >= 0) toggleDetail(S.sel);
  else if (e.key === 'g' && S.sel >= 0) judge(S.sel, 'good');
  else if (e.key === 'b' && S.sel >= 0) judge(S.sel, 'bad');
  else if (e.key === 'o' && S.sel >= 0) { const u = safeHref(S.hits[S.sel].url); if (u) window.open(u, '_blank', 'noopener,noreferrer'); }
});

/* ---------- context menu: replaces the browser's own, except where text is being edited ---------- */
const ctx = {
  el: null,
  close() { const m = $('#ctx'); m.hidden = true; m.replaceChildren(); },
  open(x, y, items) {
    const m = $('#ctx');
    m.replaceChildren(...items.map((it) => it === '-' ? h('li', { class: 'sep', role: 'separator' }) : h('li', { role: 'menuitem', tabindex: -1, onclick: () => { ctx.close(); it.run(); } }, it.label, it.hint ? h('small', { text: it.hint }) : null)));
    m.hidden = false;
    const r = m.getBoundingClientRect();
    m.style.left = Math.max(4, Math.min(x, innerWidth - r.width - 4)) + 'px';
    m.style.top = Math.max(4, Math.min(y, innerHeight - r.height - 4)) + 'px';
    const first = $('li[role=menuitem]', m); if (first) first.focus();
  },
};
document.addEventListener('contextmenu', (e) => {
  const el = e.target;
  if (el.closest && el.closest('input, textarea, select, dialog, #ctx')) return;
  e.preventDefault();
  const items = [];
  const card = el.closest && el.closest('.hit');
  const link = el.closest && el.closest('a[href]');
  const sel = String(getSelection());
  if (card) {
    const i = +card.dataset.n, x = S.hits[i], href = safeHref(x.url);
    select(i);
    if (href) items.push({ label: t('cx.open'), hint: 'O', run: () => window.open(href, '_blank', 'noopener,noreferrer') }, { label: t('cx.copylink'), run: () => copy(href) });
    items.push({ label: t('cx.copytitle'), run: () => copy(x.title || x.url) }, { label: t('cx.detail'), hint: 'D', run: () => toggleDetail(i) }, { label: t('cx.good'), hint: 'G', run: () => judge(i, 'good') }, { label: t('cx.bad'), hint: 'B', run: () => judge(i, 'bad') });
  } else if (link && safeHref(link.href)) {
    items.push({ label: t('cx.open'), run: () => window.open(link.href, '_blank', 'noopener,noreferrer') }, { label: t('cx.copylink'), run: () => copy(link.href) });
  }
  if (sel.trim()) { if (items.length) items.push('-'); items.push({ label: t('cx.copysel'), hint: 'Ctrl C', run: () => copy(sel) }); }
  if (items.length) items.push('-');
  const nav = [['search', 'nav.search'], ['read', 'nav.read'], ['status', 'nav.status']];
  if (S.desk) nav.push(['review', 'nav.review'], ['services', 'nav.services'], ['accounts', 'nav.accounts']);
  for (const [v, k] of nav) if (v !== S.view) items.push({ label: `${t('cx.go')} · ${t(k)}`, run: () => { location.hash = '#/' + v; } });
  items.push('-', { label: t('cx.reload'), run: route }, { label: t('p.theme'), run: toggleTheme }, { label: t('p.lang'), run: toggleLang }, { label: t('cx.palette'), hint: 'Ctrl K', run: openPalette });
  ctx.open(e.clientX, e.clientY, items);
});
document.addEventListener('pointerdown', (e) => { if (!e.target.closest('#ctx')) ctx.close(); });
addEventListener('blur', ctx.close); addEventListener('resize', ctx.close); document.addEventListener('scroll', ctx.close, true);
$('#ctx').addEventListener('keydown', (e) => {
  const li = $$('#ctx li[role=menuitem]'), i = li.indexOf(document.activeElement);
  if (e.key === 'Escape') { ctx.close(); e.preventDefault(); }
  else if (e.key === 'ArrowDown') { li[(i + 1) % li.length].focus(); e.preventDefault(); }
  else if (e.key === 'ArrowUp') { li[(i - 1 + li.length) % li.length].focus(); e.preventDefault(); }
  else if (e.key === 'Enter' && i >= 0) { li[i].click(); e.preventDefault(); }
});
document.addEventListener('mousedown', (e) => {
  const d = e.target;
  if (!(d instanceof HTMLDialogElement) || !d.open) return;
  const r = d.getBoundingClientRect();
  if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom) d.close('cancel');
});
boot();
