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
seesea serve --addr 127.0.0.1:8080                   # 设置 SEESEA_API_KEY 开启 Bearer 鉴权
seesea mcp                                           # stdio MCP
```

全局参数：`--searxng`、`--brave-key`、`--no-native`（不用内置 Bing/百度/搜狗抓取）、`--proxy`、`--cache-dir`、`--no-cache`、`--allow-private`、`--json`；前四项也可用 `SEESEA_SEARXNG_URL`、`SEESEA_BRAVE_API_KEY`、`SEESEA_PROXY`、`SEESEA_CACHE_DIR` 设置。

退出码：0 成功，1 其他，2 用法错误，3 配置错误或无可用来源，4 网络/上游失败。错误以 `error[code]: message` 输出。

## 已知限制

- SearXNG 与 Brave 的解析只用手写样本测试过，未对真实服务验证；请用 `seesea sources --check` 检查你的实例。
- 抓取型引擎（Bing、百度、搜狗）未在沙箱外验证，本质上也较脆弱。
- 精排目前是词法的（BM25 + 标题支撑度），尚无交叉编码器或向量模型，`Reranker` trait 是接入点。
- `deep` 模式读取更多页面，但尚未做查询改写。
- SSRF 防护检查目标与重定向，未固定 DNS 解析结果。
- 存储只有内存/文件缓存，尚未采用 Turso。

设计文档：[docs/search-pipeline-design.md](docs/search-pipeline-design.md)。许可证：AGPL-3.0-or-later。
