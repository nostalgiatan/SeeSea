# 搜索响应参考

`seesea search --json`、`GET`/`POST /v1/search` 和 MCP 的 `search` 工具返回同一种 JSON。

## 顶层字段

| 字段 | 类型 | 说明 |
|---|---|---|
| `query` | 字符串 | 查询词 |
| `mode` | 字符串 | `fast`、`standard` 或 `deep` |
| `took_ms` | 整数 | 总耗时（毫秒） |
| `hits` | 数组 | 结果，最好的在前 |
| `failures` | 数组 | 失败的来源，见下 |
| `degraded` | 字符串数组 | 这次搜索里没有正常工作的环节 |
| `stages` | 数组 | 各阶段耗时 |
| `cached` | 布尔 | 是否来自搜索缓存 |

## hits 的每一项

| 字段 | 类型 | 说明 |
|---|---|---|
| `url` | 字符串 | 结果网址 |
| `title` | 字符串 | 标题 |
| `snippet` | 字符串 | 来源给出的摘要 |
| `published` | 时间或空 | 发布时间，未知时为空 |
| `score` | 数字 | 最终分数，决定排序 |
| `score_parts` | 对象 | 分数的构成，见下 |
| `found_by` | 数组 | 哪些来源返回了它，每项是 `{source, rank}` |
| `content` | 对象或空 | 读到的正文，见下；没有读到时为空 |
| `content_error` | 字符串或空 | 没有读到正文时的原因 |
| `related` | 数组 | 同一事件的其他报道，每项是 `{url, title, host}` |
| `vet` | 对象或空 | 网址的可信度判断，见下；没有可说的（不知名的站点、没有问题）时为空 |

`related` 里的网址由标题相近且来自不同站点的结果合并而来：标题的词重合度（Jaccard）达到 0.55，或较短标题的包含度达到 0.85 且至少共有 4 个词。每多一个独立站点，主结果加 0.03 分，最多加 0.09。

### vet

| 字段 | 说明 |
|---|---|
| `level` | `trusted`、`neutral`、`suspicious`、`risky`、`blocked` |
| `tier` | 热门度档位：`giant`（前一万）、`well_known`（前十万）、`listed`（榜内其余）、`unranked` |
| `rank` | 在榜单里的名次，不在榜内时没有 |
| `official` | 这个网址属于哪个品牌自己的域名，不是时没有 |
| `flags` | 引起怀疑的信号代码，没有时省略。常见的有 `brand_in_subdomain`、`embedded_domain`、`typosquat`、`look_alike_brand`、`homograph`、`ip_host`、`shortener`、`random_name`、`risky_tld`、`path_keywords`、`threat_url`、`threat_host`、`local_block`。读到页面后还可能有 `form_elsewhere`、`brand_login_page`、`saved_copy`、`hidden_iframe`、`refresh_elsewhere`、`host_warning` |

被威胁列表或 `block` 命中的结果默认直接剔除，`degraded` 里会说明剔除了哪些主机。`risky` 的页面不会被读取，所以没有 `content`。读完页面才发现有问题的结果不会被剔除，只是排到后面并带上标记。

### score_parts

| 字段 | 层 | 说明 |
|---|---|---|
| `fusion` | 第一层 | 融合各来源名次后的分数 |
| `snippet_overlap` | 第一层 | 查询与摘要的词重合 |
| `title` | 第一层 | 查询与标题的匹配，已按标题是否被摘要或正文支撑打过折扣 |
| `freshness` | 第一层 | 新鲜度 |
| `learned` | 第一层 | 因这个主机和来源以往的表现加上的分 |
| `vet` | 第一层 | 因网址可信度加上或减去的分：热门和官方站点加分，可疑和高风险减分 |
| `stage1` | 第一层 | 第一层分数，归一到最高分为 1 |
| `relevance` | 第二层 | 正文回答查询的程度，0 到 1 |
| `title_support` | 第二层 | 正文对标题的支撑程度，0 到 1 |
| `body_quality` | 第二层 | 抽取置信度、有效字数、重复度等决定的正文质量 |

没有读到正文的结果，第二层三项都是 0。

### content

| 字段 | 说明 |
|---|---|
| `sections` | 选出的章节，每项 `{path, markdown, score}`；`path` 是从页面顶部开始的标题层级 |
| `markdown` | 整页正文，只在请求了 `full_content` 时出现 |
| `confidence` | 抽取置信度 |
| `needs_render` | 静态 HTML 里没有正文，需要执行脚本才有 |
| `truncated` | 章节因字符预算被截断 |

## failures 的每一项

| 字段 | 说明 |
|---|---|
| `source` | 来源名 |
| `kind` | `timeout`、`http`、`parse`、`blocked` 或 `other` |
| `message` | 原因 |

## stages

每项是 `{stage, ms}`。

| 阶段 | 含义 |
|---|---|
| `search` | 并发向各来源提问 |
| `fuse_rank` | 融合、去重、第一层排序 |
| `fetch_extract` | 下载并抽取页面，整个阶段 |
| `fetch_slowest_page` | 其中最慢一页的下载耗时 |
| `extract_slowest_page` | 最慢一页的抽取耗时 |
| `extract_all_pages` | 所有页面抽取耗时之和 |
| `render_slowest_page` | 需要渲染的页面里最慢的一次 |
| `rerank` | 第二层按正文重排 |
| `assemble` | 选章节、组装输出 |

页面并发下载，所以 `fetch_extract` 大致等于最慢那一页。`fast` 模式没有 `fetch_extract` 之后的阶段。

## degraded 的常见内容

| 内容 | 含义 |
|---|---|
| `N of M sources failed` | 有来源失败；细节在 `failures` |
| `returned nothing: 名字` | 通用引擎没有返回结果，其他来源却有，很可能被拦截 |
| `content: N of M pages could not be fetched or extracted (K timed out)` | 有页面读不到；`K` 是其中超时的个数 |
| `名字 skipped: it blocked us recently ...` | 该来源近期返回过验证码或限流，暂时不再询问 |
| `semantic rerank failed ...` | 嵌入模型出错，退回词法分数 |
| `rerank failed ...` | 重排任务出错，内容分数被跳过 |
