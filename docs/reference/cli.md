# 命令行参考

用法：`seesea [全局参数] <命令> [参数]`。

## 命令

| 命令 | 作用 |
|---|---|
| `search [查询...]` | 搜索并读取最好的结果 |
| `extract <目标>` | 读取一个页面：网址、本地 HTML 文件，或用 `-` 读标准输入 |
| `sources` | 列出来源；加 `--check` 时向每个来源提一个问题并报告结果 |
| `eval <文件>` | 用带标注的查询评分（JSON 行：查询和带等级的相关网址） |
| `replay <日志>` | 在 `search --log` 写出的日志上回放缓存策略 |
| `model [目录]` | 检查一个嵌入模型：加载、嵌入几个句子、报告维度、速度和相似度 |
| `config` | 显示配置文件位置；`--init` 写出带注释的模板 |
| `models` | 检查可用的模型；`models download [名称...]` 下载缺失的 |
| `users` | 管理 API 用户：`add`、`list`、`disable`、`enable`、`plan` |
| `keys` | 管理 API 密钥：`create`、`list`、`revoke` |
| `usage <用户>` | 用户每天花掉的积分 |
| `feedback <网址> good\|bad` | 告诉第一层排序这个结果好还是坏 |
| `report` | 汇总已记录的搜索 |
| `learn` | 查看或清除第一层学到的内容：`show`、`reset` |
| `feeds` | 管理为本地索引供稿的订阅源 |
| `poll` | 拉取到期的订阅源写入本地索引 |
| `index` | 本地索引的大小和日期；`prune`、`embed`、`near` |
| `cluster` | 多节点：`keygen`、`export`、`import`、`push` |
| `serve` | 运行 HTTP 服务（JSON 接口，MCP 在 `/mcp`） |
| `desk` | 打开工作台：浏览器里搜索，并能启动 HTTP 接口和远程 MCP，见[工作台](workbench.md) |
| `mcp` | 通过标准输入输出运行 MCP 服务 |

## search

| 参数 | 默认 | 说明 |
|---|---|---|
| `-m`, `--mode` | `standard` | `fast`（摘要）、`standard`（读取前几名）、`deep`（读取更多） |
| `-n`, `--limit` | 10 | 返回的结果数 |
| `--time-range` | 无 | `day`、`week`、`month`、`year` |
| `--lang` | 无 | 语言代码，如 `en`、`zh` |
| `-s`, `--sources` | 全部默认来源 | 只用这些来源，逗号分隔 |
| `--max-chars` | 3000 | 每个结果返回的正文字符上限 |
| `--full` | 关 | 返回整页而不是相关章节 |
| `--include` | 无 | 只要这些站点的结果（主机名或上级域名，逗号分隔） |
| `--exclude` | 无 | 排除这些站点 |
| `--topic` | `general` | `news` 把查询当作关于近期事件的问题 |
| `--log` | 无 | 把每次搜索以一行 JSON 追加到这个文件 |

## extract

| 参数 | 默认 | 说明 |
|---|---|---|
| `-q`, `--query` | 无 | 只返回回答这个问题的章节 |
| `--scope` | `core` | `core`，或 `extended`（同时保留评论等附近内容） |
| `--max-chars` | 6000 | 带查询时返回文字的上限 |

## sources

| 参数 | 默认 | 说明 |
|---|---|---|
| `--check` | 关 | 逐个来源提问 |
| `-q`, `--query` | `rust programming language` | `--check` 用的问题 |

## eval

| 参数 | 默认 | 说明 |
|---|---|---|
| `-k` | 10 | 指标的截断位置 |
| `-m`, `--mode` | `fast` | 评测用的搜索模式 |

输出 nDCG、召回和 MRR。

## replay

| 参数 | 默认 | 说明 |
|---|---|---|
| `--ttl` | 600 | 缓存条目的存活秒数 |
| `--fuzzy` | `0.9,0.8,0.7` | 要试的模糊匹配阈值 |

## users、keys、usage

| 命令 | 说明 |
|---|---|
| `users add <名字> [--plan 套餐]` | 建用户，套餐默认 `default` |
| `users list` | 列出用户 |
| `users disable <名字>` / `users enable <名字>` | 停用或恢复，停用后该用户所有密钥都被拒绝 |
| `users plan <名字> <套餐>` | 换套餐 |
| `keys create <用户> [--name 标签] [--expires-days N]` | 建密钥，明文只显示一次 |
| `keys list <用户>` | 列出密钥 |
| `keys revoke <id>` | 按公开 id（形如 `ss_xxxxxxxx`）吊销 |
| `usage <用户> [--days N]` | 每日积分，默认 14 天 |

## report

| 参数 | 默认 | 说明 |
|---|---|---|
| `--since` | `7d` | 回看多久，如 `90s`、`15m`、`2h`、`7d` |
| `--out` | 数据目录下的新文件夹 | 写出 `report.md`、`report.json`、`suspects.jsonl`、`candidates.jsonl` |

## feeds、poll、index

| 命令 | 说明 |
|---|---|
| `feeds add <网址>` | 直接登记一个订阅源，立即信任 |
| `feeds discover <站点> [--dry-run]` | 找出站点提供的订阅源并登记；`--dry-run` 只列出 |
| `feeds list` | 每个订阅源及其状态 |
| `feeds remove <网址>` | 移除 |
| `feeds revive <网址>` | 把停用的订阅源重新打开（进入试用期） |
| `feeds seed` | 加入内置的中英文起始订阅源（试用期） |
| `poll [--every 15m]` | 拉取到期的订阅源；带 `--every` 时按间隔重复 |
| `index` | 条数、天数和体积 |
| `index prune` | 立即执行保留策略 |
| `index embed` | 给还没有向量的条目生成向量 |
| `index near <问题> [-k 5]` | 语义上最近的条目及汉明距离（位） |

## cluster

| 命令 | 说明 |
|---|---|
| `cluster keygen <名字> --role <角色> [--out 目录]` | 为节点生成密钥对，写出 `<名字>.pem` 和 `<名字>.key`，并打印要放进其他节点 `cluster_peers` 的一行 |
| `cluster export <目录>` | 把本节点的账号（只含密钥哈希）、站点评分、索引条目和日志写进文件夹 |
| `cluster import <目录>` | 合并这样的文件夹到本节点；重复导入不会重复，站点评分的计数会累加 |
| `cluster push <目录> --to quic://主机:端口` | 推给运行中的 flywheel；本机节点密钥必须在对方名单里标为 `operator`；账号不推 |

## serve

| 参数 | 默认 | 说明 |
|---|---|---|
| `--addr` | `127.0.0.1:8080` | 监听地址 |
| `--api-key` | 无，读 `SEESEA_API_KEY` | 根密钥：除 `/health` 外每个接口都要求这个 Bearer 令牌 |
| `--max-concurrent` | 64 | 同时处理的请求上限 |
| `--allow-open` | 关 | 没有密钥和用户时仍监听非回环地址 |
| `--poll-every` | `15m` | 后台拉取订阅源的间隔；`0` 关闭 |
| `--public-url` | 无，读 `SEESEA_PUBLIC_URL` | hub 能访问到本服务的地址；设置后开启 WebSub 订阅 |

## 全局参数

全局参数可放在命令前或后。大多数也能用环境变量或配置文件设置，对应键见[配置参考](config.md)。

| 参数 | 环境变量 | 说明 |
|---|---|---|
| `--config` | `SEESEA_CONFIG` | 配置文件，默认 `~/.config/seesea/config.toml` |
| `--auth-db` | `SEESEA_AUTH_DB` | 用户和密钥所在的 SQLite 文件 |
| `--searxng` | `SEESEA_SEARXNG_URL` | SearXNG 地址（需开启 json 格式） |
| `--brave-key` | `SEESEA_BRAVE_API_KEY` | Brave Search API 密钥 |
| `--no-native` | | 不用内置来源 |
| `--proxy` | `SEESEA_PROXY` | 所有请求走这个代理：`http://`、`https://`、`socks4://` 或 `socks5://` |
| `--cache-dir` | `SEESEA_CACHE_DIR` | 缓存目录 |
| `--no-cache` | | 不用缓存 |
| `--fuzzy-cache` | `SEESEA_FUZZY_CACHE` | 词重合度（0 到 1）达到此值的查询复用缓存，默认关 |
| `--model-dir` | `SEESEA_MODEL_DIR` | 嵌入模型目录（`config.json`、`tokenizer.json`、`model.safetensors`） |
| `--semantic-weight` | `SEESEA_SEMANTIC_WEIGHT` | 内容相关度中嵌入分数的占比，默认 0.8 |
| `--semantic-passages` | `SEESEA_SEMANTIC_PASSAGES` | 每页嵌入的段落数，默认 4 |
| `--semantic-range` | `SEESEA_SEMANTIC_RANGE` | `下限,上限`：余弦相似度映射到相关度 0 和 1，默认 `0.3,0.75` |
| `--rerank-model` | `SEESEA_RERANK_MODEL` | 交叉编码器的 `.gguf` 文件 |
| `--rerank-weight` | `SEESEA_RERANK_WEIGHT` | 内容相关度中交叉编码器的占比，默认 0.5 |
| `--rerank-passages` | `SEESEA_RERANK_PASSAGES` | 交叉编码器每页评分的段落数，默认 3 |
| `--rerank-center` | `SEESEA_RERANK_CENTER` | 映射到相关度 0.5 的 logit，默认 -4 |
| `--rerank-scale` | `SEESEA_RERANK_SCALE` | 一个 sigmoid 单位对应的 logit 距离，默认 3 |
| `--lexical-sections` | `SEESEA_LEXICAL_SECTIONS` | 只按共有词挑章节，即使已加载模型 |
| `--plain-headers` | | 用朴素请求头，不模仿浏览器（`SEESEA_USER_AGENT` 可设 UA） |
| `--lightpanda` | `SEESEA_LIGHTPANDA` | 渲染 JavaScript 页面用的 Lightpanda，默认从 `PATH` 找 |
| `--no-render` | | 从不用无头浏览器渲染 |
| `--no-learn` | | 第一层排序不从以往搜索学习 |
| `--no-writeback` | `SEESEA_NO_WRITEBACK` | 读过并回答了问题的页面不写回本地索引 |
| `--no-journal` | `SEESEA_NO_JOURNAL` | 不记录搜索 |
| `--explore` | `SEESEA_EXPLORE` | 每次搜索额外读一个排名靠后结果的概率（0 到 1） |
| `--no-index` | | 不保存也不检索本地索引 |
| `--index-dir` | `SEESEA_INDEX_DIR` | 本地索引目录 |
| `--allow-private` | | 允许抓取回环和私有地址 |
| `--json` | | 输出 JSON 而不是 Markdown |

另有环境变量 `SEESEA_GITHUB_TOKEN`（提高 GitHub 来源的限额）、`SEESEA_USER_AGENT`、`SEESEA_HOST_INTERVAL_MS`（同一主机请求的最小间隔，默认 300）、`SEESEA_MODELS_DIR`、`SEESEA_EMBED_DEVICE`（`cpu` 强制不用 GPU）、`SEESEA_GPU_F32=1`、`SEESEA_EMBED_QUANTIZED=1`、`SEESEA_ROLE`。

## 退出码

| 码 | 含义 |
|---|---|
| 0 | 成功 |
| 1 | 其他错误 |
| 2 | 用法错误 |
| 3 | 配置错误或没有可用来源 |
| 4 | 网络或上游失败 |

错误以 `error[代码]: 信息` 的形式输出到标准错误。
