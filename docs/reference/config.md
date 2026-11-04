# 配置参考

## 位置与优先级

配置文件默认是 `~/.config/seesea/config.toml`，可用 `--config` 或 `SEESEA_CONFIG` 指定。`seesea config --init` 写出带注释的模板。键名写错会报错。

设置的优先级从高到低：

1. 命令行参数
2. `SEESEA_*` 环境变量
3. 配置文件
4. 在模型目录里发现的模型

## 目录

| 用途 | 默认位置 | 覆盖方式 |
|---|---|---|
| 配置 | `~/.config/seesea/config.toml` | `--config` |
| 数据 | `~/.local/share/seesea/` | `XDG_DATA_HOME` |
| 模型 | `<数据目录>/models` | `[models] dir`、`SEESEA_MODELS_DIR` |
| 用户与密钥 | `<数据目录>/auth.db` | `auth_db`、`--auth-db` |
| 本地索引 | `<数据目录>/index` | `index_dir`、`--index-dir` |
| 搜索记录 | `<数据目录>/journal`（每天一个 JSONL，保留 60 天，最多 200 MB） | |
| 第一层学到的内容 | `<数据目录>/learn.json` | |
| 渲染器 | `<数据目录>/bin/lightpanda` | `lightpanda` |
| 缓存 | 系统缓存目录（Linux 为 `~/.cache/seesea`） | `cache_dir`、`--cache-dir` |

缓存目录最多 256 MB。本地索引默认保留 30 天，上限 512 MB。

## 顶层键

| 键 | 默认 | 说明 |
|---|---|---|
| `searxng` | 无 | SearXNG 地址 |
| `brave_key` | 无 | Brave Search API 密钥 |
| `proxy` | 无 | `http://`、`https://`、`socks4://` 或 `socks5://` |
| `no_native` | `false` | 关闭内置来源 |
| `allow_private` | `false` | 允许 `extract` 访问回环和私有地址 |
| `plain_headers` | `false` | 抓取页面时不模仿浏览器 |
| `lexical_sections` | `false` | 只按共有词挑章节 |
| `lightpanda` | 从 `PATH` 找 | 渲染 JavaScript 页面的程序 |
| `no_render` | `false` | 不渲染 |
| `no_learn` | `false` | 第一层不学习 |
| `no_journal` | `false` | 不记录搜索 |
| `device` | `gpu` | `cpu` 让模型完全不用 GPU |
| `model_idle_minutes` | 10 | 模型空闲这么久后卸载；0 表示不卸载 |
| `no_writeback` | `false` | 读过并回答了问题的页面不写回本地索引 |
| `explore` | 无 | 每次搜索额外读一个排名靠后结果的概率 |
| `fuzzy_cache` | 关 | 近似查询复用缓存的词重合度阈值 |
| `cache_dir` | 见上 | |
| `no_index` | `false` | 不保存也不检索本地索引 |
| `index_dir` | 见上 | |
| `retention_days` | 30 | 索引保留天数 |
| `index_quota_mb` | 512 | 索引总量上限 |
| `auth_db` | 见上 | |
| `shared_limits` | `true` | 令牌桶和日额度放在用户库里，多个 `serve` 进程共享；`false` 改回进程内存 |

## `[collect]`

除新闻外，本地索引还能收集几类数据。开关默认全关；打开后对应订阅源以试用状态加入，失效的会自动停用。已收集的内容在开关关闭后仍可检索。环境变量 `SEESEA_COLLECT`（逗号分隔）优先于配置文件。

| 键 | 内容 |
|---|---|
| `dev` | GitHub 发布与标签、PyPI 更新、安全公告（CISA、Ubuntu、Debian、GitHub Advisory） |
| `papers` | arXiv 各学科每日列表（cs.CL、cs.LG、cs.AI、cs.CV、cs.IR、cs.CR、cs.SE、cs.DC、stat.ML） |
| `community` | Hacker News、Lobsters、Stack Overflow 标签、Reddit、V2EX |
| `quotes` | 汇率、币价、天气：被问到时才向公开接口查询，不入库，结果缓存 30 秒。每种值有多个备用接口，按顺序取第一个能答的 |

## TLS

监听非回环地址时，要么配置证书，要么声明前面有终结 TLS 的代理。`--allow-open` 不能绕过这一条。

| 键 | 说明 |
|---|---|
| `tls_cert`、`tls_key` | 服务证书和私钥 |
| `behind_tls_proxy` | `true` 表示前面的反向代理终结 TLS |
| `tls_client_ca` | 同时要求客户端出示这个 CA 签发的证书 |
| `trust_forwarded_for` | 在设置 `X-Forwarded-For` 的代理后面时，从它的最后一项取客户端地址 |

## 套餐

套餐写在 `[plans.名字]` 表里。`default` 和 `unlimited` 已内置。

| 键 | 说明 |
|---|---|
| `per_minute` | 每分钟的积分 |
| `burst` | 令牌桶容量 |
| `per_day` | 每日积分 |
| `max_concurrent` | 同时处理的请求数 |
| `deep` | 是否允许 `deep` 模式 |

`default` 为每分钟 60 积分、突发 30、每日 5000、并发 4。

```toml
[plans.free]
per_minute = 20
burst = 10
per_day = 500
max_concurrent = 2
deep = false
```

## [models]

| 键 | 说明 |
|---|---|
| `dir` | 模型目录 |
| `embedding` | 嵌入模型文件，如 `bge-m3-Q4_0.gguf` |
| `reranker` | 交叉编码器文件，如 `bge-reranker-v2-m3-Q4_K_M.gguf` |

没有写路径时，`seesea` 在模型目录里按固定的文件名查找。文件名以 `seesea models` 的输出为准，大小写要一致。

## [rerank] 和 [semantic]

| 键 | 默认 | 对应参数 |
|---|---|---|
| `[rerank] weight` | 0.5 | `--rerank-weight` |
| `[rerank] passages` | 3 | `--rerank-passages` |
| `[rerank] center` | -4.0 | `--rerank-center` |
| `[rerank] scale` | 3.0 | `--rerank-scale` |
| `[semantic] weight` | 0.8 | `--semantic-weight` |
| `[semantic] passages` | 4 | `--semantic-passages` |
| `[semantic] range` | `"0.3,0.75"` | `--semantic-range` |

交叉编码器的输出按 `sigmoid((logit - center) / scale)` 映射成相关度。

## 多节点

| 键 | 用于 | 说明 |
|---|---|---|
| `role` | 所有节点 | `single`（默认）、`gateway`、`search`、`flywheel`、`witness`；也可用 `SEESEA_ROLE` |
| `node_cert`、`node_key` | 所有节点 | 本节点的证书和私钥，由 `cluster keygen` 生成 |
| `cluster_peers` | 所有节点 | 对端名单，每行 `<SHA-256 指纹> <角色> <名字>`；改动后自动重读 |
| `cluster_listen` | 搜索、flywheel、见证节点 | 监听的 UDP 地址，如 `0.0.0.0:7443` |
| `search_nodes` | 网关 | 搜索节点地址列表，`quic://主机:端口` |
| `flywheel` | 网关、搜索节点 | 主 flywheel 地址 |
| `flywheel_standby` | 网关、搜索节点 | 备用 flywheel 地址 |
| `standby_of` | 备用 flywheel | 复制的主节点地址（一主一备，无见证时用） |
| `partner`、`witness` | 有见证的两个 flywheel | 另一个 flywheel 和见证节点的地址 |

`cluster_peers` 里的角色是 `gateway`、`search`、`flywheel`、`witness` 或 `operator`。

## 环境变量

除[命令行参考](cli.md#全局参数)里列出的以外：

| 变量 | 说明 |
|---|---|
| `SEESEA_API_KEY` | 根密钥，等同 `serve --api-key` |
| `SEESEA_PUBLIC_URL` | 等同 `serve --public-url` |
| `SEESEA_GITHUB_TOKEN` | 提高 GitHub 来源的限额 |
| `SEESEA_USER_AGENT` | 抓取页面用的 User-Agent |
| `SEESEA_HOST_INTERVAL_MS` | 同一主机两次请求的最小间隔，默认 300，本机和内网地址不限 |
| `SEESEA_MODELS_DIR` | 模型目录 |
| `SEESEA_EMBED_DEVICE` | `cpu` 强制不用 GPU |
| `SEESEA_GPU_F32` | 设为 `1` 时 GPU 上的权重用 f32，默认 f16 |
| `SEESEA_EMBED_QUANTIZED` | 设为 `1` 时保留 4 位权重，内存约少一半，分数低约 0.01，更慢 |
