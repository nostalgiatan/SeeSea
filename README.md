# SeeSea

[English](./README_EN.md) | 中文

给 AI 用的统一检索管线，一个高性能的单文件二进制。查询扇出到多个来源，融合排序，抓取前几名页面，按**正文**重排（标题党只有被正文支撑才算分），最后在字符预算内只返回相关章节。

可作为命令行工具、HTTP 服务（`/v1/search`、`/v1/extract`）或 MCP 服务（stdio 或 `/mcp`）使用。

## 构建

```bash
cd seesea-core
cargo build --release -p seesea-cli      # 产物：target/release/seesea
```

## 使用

```bash
export SEESEA_SEARXNG_URL=http://localhost:8888      # 自建 SearXNG（需开启 json 格式）
export SEESEA_BRAVE_API_KEY=...                      # 可选，Brave Search API

seesea search "rust async runtime"                   # standard：读取前几条结果正文
seesea search -m fast -n 5 --json "query"            # 仅摘要
seesea search -m deep --time-range week "query"
seesea extract https://example.com/post -q "thread pool" --max-chars 1500
seesea extract page.html                             # 本地文件，或用 `-` 读标准输入
seesea sources --check                               # 逐个来源探测
seesea eval judgments.jsonl -k 10                    # nDCG / recall / MRR
seesea search --log s.jsonl "query"                   # keep a search log
seesea replay s.jsonl --ttl 600                      # cache policies on that log: hit ratio vs result drift
seesea serve --addr 127.0.0.1:8080                   # 设置 SEESEA_API_KEY 开启 Bearer 鉴权
seesea config --init                                 # 写出带注释的 ~/.config/seesea/config.toml
seesea models                                       # 查看模型与渲染器是否就绪
seesea models download                              # 下载缺失的 GGUF 模型（ModelScope），之后自动使用
seesea model ./bge-small-en-v1.5                     # 检查嵌入模型能否加载、能否区分相关/无关
seesea --model-dir ./bge-small-en-v1.5 search "query"  # 语义精排（纯 Rust 推理，模型需自行下载）
seesea --rerank-model ./bge-reranker-v2-m3-q4_k_m.gguf search "query"  # 交叉编码器精排（可与 --model-dir 叠加）
seesea feedback https://example.org/a bad            # 告诉第一层排序这个结果好/坏（HTTP：POST /v1/feedback）
seesea learn show                                    # 第一层学到的各主机/来源表现；`learn reset` 清空
seesea feeds seed                                   # 加入内置的中英文起始订阅源（试用期，坏的会自动停用）
seesea feeds add https://example.org/feed.xml        # 或 `feeds discover example.org` 自动发现站点的订阅源
seesea poll --every 15m                              # 拉取订阅源写入本地索引（`serve` 默认每 15 分钟在后台做）
seesea index                                         # 本地索引的条数、天数和体积
seesea mcp                                           # stdio MCP
```

设置来源按优先级依次为：命令行参数、`SEESEA_*` 环境变量、`~/.config/seesea/config.toml`（可用 `--config`/`SEESEA_CONFIG` 指定；写错键名会报错）、模型目录（`~/.local/share/seesea/models`）里找到的模型。`seesea config --init` 会写出带注释的模板。

全局参数：`--searxng`、`--brave-key`、`--no-native`（不用内置 Bing 抓取）、`--proxy`、`--cache-dir`、`--no-cache`、`--allow-private`、`--json`；前四项也可用 `SEESEA_SEARXNG_URL`、`SEESEA_BRAVE_API_KEY`、`SEESEA_PROXY`、`SEESEA_CACHE_DIR` 设置。

退出码：0 成功，1 其他，2 用法错误，3 配置错误或无可用来源，4 网络/上游失败。错误以 `error[code]: message` 输出。

## 使用记录与自动检查

每次搜索（命令行、HTTP、MCP）和每条反馈都会自动记入数据目录下的 `journal/`，每天一个 JSONL 文件，保留 60 天；用 `--no-journal` 或配置里的 `no_journal = true` 关闭。记录里有查询、各结果的排名与分数构成、来源失败原因、各阶段耗时、读不了的页面。

```bash
seesea report --since 7d      # 汇总，并写出 report.md、report.json、suspects.jsonl、candidates.jsonl
tools/auto/run.sh             # 构建、测试、离线评测、来源探测、报告，全部落到 runs/<时间>/，并与上一次对比
tools/auto/judge.py candidates.jsonl >> tools/search-eval/live-judgments.jsonl
```

`suspects.jsonl` 是看起来有问题的搜索（无结果、来源全败、顶部分数低、页面多数读不了、过慢）。`candidates.jsonl` 是每个查询的前十个结果，`grade` 为空；填 0–3 后用 `judge.py` 转成评测文件，`run.sh` 之后会自动对真实网页再评一遍。

## 借鉴其他搜索服务的做法

- **自建索引（博查的思路）**：读过并且确实回答了问题的页面（重排序相关度 ≥ 0.5 且正文质量 ≥ 0.4）会写回本地索引，只存标题和开头，不存全文；下次搜索由 `local` 来源直接召回，网络断开也能用。写回的页面没有发布日期，不会被当成新内容。用 `--no-writeback` 关闭。
- **按域名过滤、新闻主题（Tavily）**：`--include`、`--exclude`、`--topic news`；HTTP 和 MCP 同名参数 `include_domains`、`exclude_domains`、`topic`。
- **批量搜索（AnySearch）**：`POST /v1/search/batch`，`queries` 最多 10 条，同时跑 4 条，每条单独计费，互不影响。
- **相似页面（Exa 的 findSimilar）**：`POST /v1/similar`，给 `url` 或 `text`，按语义在本地索引里找相近的页面；需要索引和嵌入模型。
- 已有的：多源并发和融合、正文抽取、按问题挑段落（对应 Exa 的 highlights）、交叉编码重排序（对应 Jina 的 reranker）。

## 内存与资源

- **设备**：模型默认在 GPU（Vulkan）上跑：CPU 版需要约 3.8 GB 内存，GPU 版权重放进显存、内存只占几百 MB。没有可用显卡、显存不足或显卡丢失时自动改用 CPU 并打印一行提示。想让模型完全不碰 GPU：配置里写 `device = "cpu"` 或设 `SEESEA_EMBED_DEVICE=cpu`。GPU 上权重以 f16 存放（显存减半；`SEESEA_GPU_F32=1` 改回 f32），注意力用 burn 内置的融合内核，空闲卸载时显式释放显存。
- **向量缓存与流水线**：语义精排前面有一层向量缓存（最多 8192 条，约 32 MB，最旧的先淘汰；问题和段落分开存），同一个页面再次出现不重新编码；抓到一个页面就立刻在后台编码它的段落（`Reranker::prepare`），与其余页面的下载重叠，精排时直接取缓存。离线评测上重复查询的精排阶段 427→149 毫秒，37 条查询的评测 130→100 秒，nDCG 不变（0.851）；真实网络下重叠的收益取决于下载比编码慢多少，未实测。
- **模型的加载与卸载**：服务启动时在后台预加载，搜索繁忙时一直保留；空闲超过 `model_idle_minutes`（默认 10 分钟，0 表示不卸载）就释放并把内存还给系统，下一次搜索再自动加载（搜索一开始就在后台重新加载模型，与抓取重叠，GPU 上实测整次搜索 2.9 秒，热状态 0.5–0.8 秒；显存回到加载前的水平，进程内存 1.6 GB → 0.6 GB）。
- **内存量**：bge-m3 和 reranker 各约 1.4 GB（f32）。`SEESEA_EMBED_QUANTIZED=1` 保留 4 位权重，约少一半，评测分数低约 0.01，更慢。
- **无头浏览器（Lightpanda）**：服务启动时探测一次（渲染一个本地的、必须执行脚本才能出现文字的页面），之后正常时每 5 分钟、异常时每 30 秒复查。连续 3 次失败就暂停渲染（30 秒起指数退避，最长 10 分钟），暂停期间需要渲染的页面直接用静态结果，不再等满超时；恢复后自动继续。页面本身加载失败不算浏览器故障。
- 搜索缓存目录最多 256 MB，过期条目和中断写入的残留会被清理；搜索记录目录最多 200 MB、60 天；本地索引默认 512 MB、30 天。
- 服务收到 Ctrl-C 或 SIGTERM 时，先处理完在途请求，再写出用量和学习到的权重。

## 已知限制

- 实时层：问题按时效分三档（`Recency`：常青 / 近期 / 突发，看“最新、今天、breaking、年份”等词），时效权重和半衰期随档位变化（突发半衰期半天、权重 ×8；近期 14 天、×4；常青 365 天、×1）。`bingnews`（Bing 新闻 RSS）只在非常青的问题里自动参与，点名 `-s bingnews` 则总是参与。本地索引（`local` 来源）存标题、约 400 字的导语、日期、主机，不存正文（正文由流水线实时抓取，所以索引很小）；每天一个 SQLite 文件，标题加权的 FTS5 全文索引（中文按字拆分，查询用相邻两字），按 URL 归一化和“同站同标题”去重，默认保留 30 天、总量上限 512 MB（先删最旧的一天）。填充方式：RSS 2.0/RSS 1.0/Atom/JSON Feed 轮询（失败指数退避，试用期 3 次失败、正式 6 次失败后停用，间隔随有无新条目自适应 5 分钟～6 小时，并发 6、单个 2 MB 上限、15 秒超时，任何一个源出错都只影响它自己）；`POST /v1/ingest` 推送（每次最多 1000 条）；来源自动发现：搜索结果里出现过的站点会排队，轮询时每轮最多探 3 个站点（首页的 `<link rel=alternate>`，没有就试 /feed、/atom.xml 等常见位置，首页没有链接且装有 Lightpanda 时渲染后再找），发现的订阅源先进试用期。补充：新闻站点地图（Google News sitemap，含标题和发布时间，来自 robots.txt 里名字带 news 的条目或常见路径）也当订阅源用。**向量**：条目入库后由轮询按批（每轮 256 条）用嵌入模型生成向量，取“减去首批均值后的符号位”存成每条 128 字节（1024 维），检索时对近期各天的向量做汉明距离暴力扫描（四成二位以内才算相近），与全文结果做 RRF 合并，所以英文问题能找到中文报道（实测 bge-m3：“refrigerators hacked in supermarkets”最近的是中英文三篇同一事件，距离 316–364 位，无关内容约 430–460 位）；`seesea index embed` 立即补齐，`seesea index near "问题"` 查看最近邻与距离。`fast` 模式不加载模型，所以不用向量。**事件聚合**：标题相近（Jaccard ≥ 0.55，或短标题的包含度 ≥ 0.85）且来自不同站点的结果并为一条，其余列在 `related` 里，每多一个独立站点加 0.03 分（最多 0.09）；跨语言的同一事件不会合并。**排序**：本地索引先在各天文件里取候选，再用跨天累计的文档频率统一打分（标题命中算三倍的导语命中），不受条目落在哪一天的影响。**推送**：条目所在订阅源声明了 WebSub hub 时，`serve --public-url https://你的域名`（或 `SEESEA_PUBLIC_URL`）会向 hub 订阅，回调为 `/v1/websub/{id}`（不需要 API 密钥，靠挑战回显与 HMAC-SHA256/512 签名校验）；到期前自动续订，轮询仍作为兜底。WebSub 只用单元测试和本地 HTTP 测试验证过，没有对真实 hub 测过（需要公网地址）。**错误语义**：本地索引标为辅助来源，所有真实来源都失败而索引没有命中时仍然报 `all_sources_failed`；索引有命中则返回并标注降级。未做：SSE/WebSocket 流式订阅（没有常见的公开来源可用，需要时用 `/v1/ingest` 推送）、站内外链挖掘（只有“搜索结果里出现过的站点”这一路按需发现）、跨语言的事件合并。内置起始订阅源是我按已知地址列的，部分在你的网络里可能连不上，可用 `seesea feeds list` 看每个源最近一次的错误。

- 第一层排序会从日志学习：每次读了正文之后，第二层给出的相关度（0.7）与正文质量（0.3）记到该主机和来源名下（取不到正文记 0.15），指数衰减（半衰期 90 天）、向全局均值收缩、至少约 3 次观察后才生效，给第一层得分加一个有界的修正（主机最多约 ±0.075，来源 ±0.025；`score_parts.learned` 可见）。`feedback good/bad` 相当于 5 次观察。状态存在 `~/.local/share/seesea/learn.json`（原子写，服务进程每 30 秒落盘）；`--no-learn` 关闭，`eval` 不学习。偏差：只有被读取的候选才有观察，所以 `explore`（每次搜索以该概率换读一个排名靠后的结果）用来缓解。限制：未在标注集上证明能提高 nDCG（8 个查询的集合已饱和）；同一查询的缓存响应不会随学习更新；主机表上限 2 万。

- SearXNG 与 Brave 的解析只用手写样本测试过，未对真实服务验证；请用 `seesea sources --check` 检查你的实例。
- 内置来源无需账号：Bing、DuckDuckGo（HTML）、Mojeek、Wikipedia 默认参与；Stack Overflow、arXiv、GitHub 只回答特定类型的问题，需要点名才使用（`-s stackoverflow,arxiv,github`；`SEESEA_GITHUB_TOKEN` 可提高 GitHub 限额）。百度、搜狗已移除（验证码/超时）。抓取型来源本质上脆弱：DuckDuckGo、Mojeek 常对云主机和部分家庭网络返回验证码（报告为 `Blocked`），部分网络下 Wikipedia/DuckDuckGo 可能无法访问。只有 arXiv、Stack Overflow、GitHub 对真实服务验证过，DuckDuckGo、Mojeek、Wikipedia 的解析基于样本，请运行 `seesea sources --check`。更稳的来源是 SearXNG 或 Brave API。`--no-native` 关闭全部内置来源。
- 抓取页面时默认带浏览器式请求头（User-Agent、Accept-Language、Sec-Fetch-*、客户端提示，三套互相一致的浏览器画像；同一站点固定用同一套）；遇到 403/429/503 会换一套画像并带上搜索引擎 Referer 重试，最多共 3 次，遵守 Retry-After。只改请求头：不模拟 TLS 指纹、不执行脚本、不破解验证码，所以需要这些的站点（如知乎、百度百科）仍会拒绝。`SEESEA_USER_AGENT` 可自定义 UA，`--plain-headers` 关闭。同一主机的请求之间默认至少间隔 300 毫秒并带随机抖动（`SEESEA_HOST_INTERVAL_MS` 调整，本机和内网地址不限速）。隐私模块已重写（原来的 TLS/Canvas 指纹“混淆”、Tor 电路管理和隐私评分只是摆设，已删除）；要隐藏出口地址请用 `--proxy`，例如指向 Tor 的 SOCKS5。
- 静态 HTML 里没有正文的页面（脚本渲染的应用）会用 [Lightpanda](https://github.com/lightpanda-io/browser) 渲染，前提是它在 `PATH` 中、在 `seesea models download renderer` 的安装位置（数据目录下的 `bin/lightpanda`），或由 `--lightpanda`/`SEESEA_LIGHTPANDA`/配置里的 `lightpanda` 指定；`--no-render` 关闭。结果带 `rendered: true`。浏览器被禁止连接私有地址（包括页面脚本发起的请求），不能配合 SOCKS 代理，也无法越过拒绝 seesea 的服务器（知乎仍返回 403）。渲染结果用独立的键缓存。
- SSRF：抓取前会解析主机名，只要有任何一个地址是私有的（回环、RFC 1918、含 169.254.169.254 的链路本地、ULA）就拒绝。检查刻意宽松，避免用户网络环境导致完全无法使用：`198.18.0.0/15`（TUN 类代理的 fake-IP 段）视为公网，解析失败或超时不拦截，使用 `--proxy` 时由代理解析，因此不检查。检查发生在连接之前而不是连接时，所以检查与连接之间改变解析结果的域名拦不住。
- 旧的 API 服务、RSS、热点、向量库、事件、raming 各 crate，React 前端，`openapi.json` 以及旧的隐私配置类型已删除（破坏性变更）。HTTP 服务有用户/API 密钥/套餐/限流（`seesea users`、`keys`、`usage`，见配置文件的 `[plans]`）：密钥形如 `ss_<id>_<secret>`，库里只存 SHA-256；令牌桶按 fast 1 / standard 2 / deep 5 积分扣费，并有日额度、并发上限和按 IP 的失败限速；限流状态（令牌桶、日额度）默认放在用户库里，同一个 `auth_db` 上的多个 `serve` 进程共享（配置 `shared_limits = false` 改回进程内存）；并发上限和按 IP 的失败计数仍是每进程的。**账号接口**：`GET /v1/account`（自己的套餐、今日用量、30 天用量、密钥列表）、`POST /v1/account/keys`、`DELETE /v1/account/keys/{id}`（每人最多 20 把有效密钥，别人的密钥按“不存在”处理）；根密钥（`--api-key`）另有 `GET/POST /v1/admin/users`、`PATCH /v1/admin/users/{name}`（套餐、停用）、`POST /v1/admin/users/{name}/keys`、`DELETE /v1/admin/keys/{id}`，开放（无密钥）的服务器不提供管理接口。**同机多进程**（如不停机重启）：用户、密钥、限流共享同一个 SQLite 文件，吊销和停用立即对所有进程生效；订阅源轮询靠索引目录里的租约只让一个进程执行，持有者停止续约后其他进程接手。**多节点（可选，默认单节点）**：`role = "single"`（默认）一个进程做所有事。需要扩展时给每个节点一个身份（配置 `role` 或 `SEESEA_ROLE`）：**gateway**（网关）是面向用户的节点（对用户仍是 HTTP），**不保存任何账号**：校验密钥、扣积分都通过 wire 问 flywheel（`Admit`/`Charge`/`Refund`，账号、套餐、令牌桶都在 flywheel 的 `auth_db` 里），所以可以同时运行任意多个网关放在负载均衡后面；`/v1/account*`、`/v1/admin*` 原样转给 flywheel，由它自己校验密钥；把搜索转发给 `search_nodes` 里健康的节点（每 5 秒 ping，连不上的跳过，对方说“忙”的也跳过，最多试 3 个），`/v1/ingest`、`/v1/index`、WebSub 回调转给 flywheel，自己不加载模型；**search**（搜索节点）跑流水线和模型，只回应网关，自己不存东西：读到的有用页面、使用记录、学到的站点评分都批量推给 flywheel，“本地索引”来源改为向 flywheel 查询，每分钟拉取一次合并后的站点评分；**flywheel**（数据飞轮节点）保存索引（订阅源轮询、写回的页面、向量）、合并后的站点评分（各节点的观测直接相加）和全部节点的使用记录，`seesea report` 在它上面跑就是全局视图。

**节点间协议 `seesea-wire/1`（QUIC 之上的应用层协议，不是 HTTP，也不是裸 TCP）**：传输用 QUIC（UDP，每条连接内置 TLS 1.3，所以天然没有明文；各请求走各自的流，互不阻塞；丢包恢复、连接迁移由 QUIC 负责），ALPN 名 `seesea/1` 就是版本号。帧格式 `tag:u8 | 头长:u32 | 体长:u32 | 头(JSON) | 体(原始字节)`，一个请求占一条双向流；请求的 tag 是操作（`Ping`、`Events`、`LearnGet`、`LearnMerge`、`IndexSearch`、`Http`——网关转发的用户请求），回复的 tag 是状态（`Ok`、`Error`、`Busy`、`Forbidden`）。**身份是每个节点自己的密钥对，没有共享密钥**：`seesea cluster keygen NAME --role search` 生成自签证书和私钥，并打印一行 `<SHA-256 指纹> <角色> <名字>`；把各节点的这一行放进对方的 `cluster_peers` 文件（`gateway|search|flywheel|operator`），配置 `node_cert`、`node_key`、`cluster_peers`，搜索/flywheel 节点再配 `cluster_listen = "0.0.0.0:7443"`（UDP），地址写 `quic://host:7443`。握手时双方都必须出示证书且指纹在对方名单里；客户端还要求对方是它期望的角色（连 flywheel 的必须是 flywheel）。**权限看名单里的角色，不看对方自称**：search 只能发事件、读合并后的评分、查索引；gateway 只能转发用户请求；operator（人拿着节点密钥做迁移）能推数据和合并评分；谁都不能越权。**吊销**：从 `cluster_peers` 删掉一行即可，文件改动后自动重读，已建立的连接在下一个请求时被关闭。**背压**：节点并发满了立刻回 `Busy`（什么都没做），网关换下一个节点；全满则回 503 + `Retry-After`。**可靠投递**：事件带 `(epoch, seq)` 编号，flywheel 记住每个发送者已应用到哪，重发的批次只应用一次；search 节点内存队列满了（2 万条或 64 MB）就写到 `spool` 目录（每个文件约 8 MB，最多 2 GB，重启后继续发送），超过才丢最旧的并计数。HTTP 的监听（用户那边）仍要求 TLS：监听非回环地址必须配 `tls_cert` + `tls_key`，或声明 `behind_tls_proxy = true`（`--allow-open` 也不能绕过）；在反向代理后面时设 `trust_forwarded_for = true`。QUIC 的 TLS 1.3、证书指纹校验用的是 rustls 和 quinn，应用层只定义上面的帧、操作和权限。**迁移**：`seesea cluster export 目录` 把单节点的账号（只含密钥哈希）、站点评分、索引条目、使用记录写成一个文件夹；`seesea cluster import 目录` 合并进本机（重复导入不会重复，站点评分的计数会累加）；`seesea cluster push 目录 --to quic://flywheel:7443` 推给运行中的 flywheel（用本机配置的节点密钥，需在 flywheel 的名单里标为 `operator`；账号不推，在网关机器上 `import`）。已有的密钥在新节点上照常可用。**flywheel 主备**：再起一个 flywheel 节点，配 `standby_of = "quic://主:7443"`，它每 5 秒拉主节点的账号（用户、密钥、套餐、停用/吊销），每分钟拉合并后的站点评分，每 10 分钟拉一遍索引条目（`Replicate` 操作，两个 flywheel 在对方名单里互相列为 `flywheel`）。网关和搜索节点配 `flywheel_standby = "quic://备:7443"`：始终先找主，主连不上就换备（失败的节点跳过 10 秒，不会每次等连接超时），事件、账号校验、`/v1/ingest` 等都一样。主恢复后客户端又会优先找主，但主在停机期间错过了备上发生的写入：**故障转移后请把备当作新的主**（旧主改配 `standby_of` 指向它，再把两边的 `flywheel`/`flywheel_standby` 对调）。这是异步复制，不是共识：主挂掉的瞬间最近 5 秒内的账号改动（新建的密钥、套餐变更）和令牌桶状态可能还没到备上；用量计数在备上从复制到的值继续；日志（journal）不复制，报告在故障转移后只覆盖转移之后加上备收到的部分。备也会轮询订阅源（重复条目无害，只是多抓一份）。**账号在 flywheel 上管理**：`seesea users`/`keys` 在 flywheel 所在机器上运行，`cluster import` 的账号也导到那里（套餐 `[plans]` 配在 flywheel 的配置里；备要配一样的）。多个网关各自的“并发上限”和“失败密钥计数”是各网关本地的（同一用户同时在 N 个网关上最多 N 倍并发）；每个请求多一次 flywheel 往返。**未做**：自动切回、脑裂防护（主备都活着但网络分区时，两边会各自接受写入，没有仲裁）、多于一个备；**多节点测试**：`scripts/netns-cluster.sh target/debug/seesea` 在一台机器上用无特权的用户+网络命名空间（不需要 root 或 Docker）起 5 个各有独立 IP 的节点（主备 flywheel、2 个搜索节点、网关），用虚拟交换机加 2 ms 延迟，然后 kill -9 主 flywheel、kill -9 搜索节点、30% 丢包、网络分区，检查账号、搜索、故障转移是否照常；没有真实多机/跨机房的测试。
没有注册页、计费和控制台。
- 提取器：表格按 DOM 还原为 Markdown 表格（colspan/rowspan 和空单元格保持列对齐，单列的布局表格保持段落），上下标转为 Unicode（m²、10⁻¹、H₂O）；短小的推广块（“点击报名”“buy now”等）被剔除；链接 href 里被 CMS 写进去的 Markdown 会还原为真实地址。章节选择不会留下“只有标题没有正文”的孤立标签。
- 模型也可以是单个 `.gguf` 文件（BERT/XLM-R 布局，如 bge-m3 的 Q4_0，约 400 MB）：纯 Rust 读取并解码，线性层解成 f32 常驻内存（约 1.5 GB）。在有 Vulkan GPU 的机器上（默认构建已包含，纯 Rust：burn + wgpu），24 层 Transformer 跑在 GPU 上：RTX 5070 Laptop 上 32 个约 280 词元的段落 3.5 秒（CPU 约 50 秒），相似度与 CPU 一致到小数点后三位；没有可用 GPU 时自动退回 CPU（`SEESEA_EMBED_DEVICE=cpu` 可强制）。模型在后台线程加载（约 1.7 秒，含 GPU 内核预热），与网络检索重叠，所以不拖慢单次搜索；`fast` 模式不加载模型。默认参数（权重 0.8、余弦映射 0.3–0.75）按 bge-m3 在 `tools/search-eval` 上调过。带 GPU 的二进制约 47 MB，`cargo build --no-default-features` 约 15 MB。bge-m3 质量好且跨语言（英文查询可命中中文段落）。
- 交叉编码器精排：`--rerank-model` 指向 bge-reranker-v2-m3 的 `.gguf`（Q4_K_M 约 440 MB，与 bge-m3 同属 XLM-R，复用同一套纯 Rust 读取器和 GPU 路径，加载约 1.8 秒，后台进行）。它把查询和段落放在一起读，比比较向量更能区分“谈到这个话题”和“回答了这个问题”；每页只评分开头段和最匹配的几段（`--rerank-passages`，默认 3）。输出的原始 logit 经 `sigmoid((logit-center)/scale)` 映射（`--rerank-center` 默认 -4，`--rerank-scale` 默认 3：模型很严格，答案也常在 -4 附近），再按 `--rerank-weight`（默认 0.5）与下层得分（嵌入或词法）混合。用 `tools/search-eval` 在 `--mode deep` 下的 nDCG@5：仅词法 0.828，bge-m3 0.952，仅交叉编码器 0.945，两者叠加 0.970；这个集合只有 8 个查询且已接近饱和，差异不应过度解读。
- 章节选择：页面放不下预算时，先用词法挑出最有希望的 12 个片段，再交给已加载的模型（优先交叉编码器，否则嵌入）按语义重新打分（0.3 词法 + 0.7 语义），这样答案章节即使和查询没有共同词也能被选中；`--lexical-sections` 可关闭。`extract -q` 带查询时也会加载模型。
- 反堆砌：词法、嵌入和交叉编码器三种打分都乘以“页面内容的非重复度”（不同词三元组的占比，重复同一短语的页面趋近 0），否则一个把查询重复几十遍的页面在三者眼里都是“完美匹配”。
- 语义精排需要 `--model-dir` 指向 BERT 类嵌入模型（config.json、tokenizer.json、model.safetensors，如 BAAI/bge-small-en-v1.5；可用 `seesea-embed.json` 设置 pooling 和前缀），基于 candle 在 CPU 上推理，无 C/C++ 运行时。不提供模型时退回词法精排（BM25 + 标题支撑度）。余弦到分数的映射（`cos_floor`/`cos_ceil`）按 BGE 类模型设定，尚未在标注集上验证。
- `deep` 模式读取更多页面，但尚未做查询改写。
- SSRF 防护检查目标与重定向，未固定 DNS 解析结果。
- 缓存：查询按有意义的词归一化（大小写、词序、停用词不影响命中）；页面缓存按每个 URL 的变化情况自适应延长或缩短有效期；`--fuzzy-cache 0.8`（默认关）让相近查询复用结果，先用 `seesea replay` 看代价；服务端 `GET /v1/stats` 给出命中率。
- 存储只有内存/文件缓存，尚未采用 Turso。

设计文档：[docs/search-pipeline-design.md](docs/search-pipeline-design.md)。许可证：AGPL-3.0-or-later。
