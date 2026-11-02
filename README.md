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
seesea model ./bge-small-en-v1.5                     # 检查嵌入模型能否加载、能否区分相关/无关
seesea --model-dir ./bge-small-en-v1.5 search "query"  # 语义精排（纯 Rust 推理，模型需自行下载）
seesea mcp                                           # stdio MCP
```

全局参数：`--searxng`、`--brave-key`、`--no-native`（不用内置 Bing/百度/搜狗抓取）、`--proxy`、`--cache-dir`、`--no-cache`、`--allow-private`、`--json`；前四项也可用 `SEESEA_SEARXNG_URL`、`SEESEA_BRAVE_API_KEY`、`SEESEA_PROXY`、`SEESEA_CACHE_DIR` 设置。

退出码：0 成功，1 其他，2 用法错误，3 配置错误或无可用来源，4 网络/上游失败。错误以 `error[code]: message` 输出。

## 已知限制

- SearXNG 与 Brave 的解析只用手写样本测试过，未对真实服务验证；请用 `seesea sources --check` 检查你的实例。
- 抓取型引擎（Bing、百度、搜狗）未在沙箱外验证，本质上也较脆弱。
- 模型也可以是单个 `.gguf` 文件（BERT/XLM-R 布局，如 bge-m3 的 Q4_0，约 400 MB）：纯 Rust 读取并解码，线性层解成 f32 常驻内存（约 1.5 GB）。在有 Vulkan GPU 的机器上（默认构建已包含，纯 Rust：burn + wgpu），24 层 Transformer 跑在 GPU 上：RTX 5070 Laptop 上 32 个约 280 词元的段落 3.5 秒（CPU 约 50 秒），相似度与 CPU 一致到小数点后三位；没有可用 GPU 时自动退回 CPU（`SEESEA_EMBED_DEVICE=cpu` 可强制）。模型在后台线程加载（约 1.7 秒，含 GPU 内核预热），与网络检索重叠，所以不拖慢单次搜索；`fast` 模式不加载模型。默认参数（权重 0.8、余弦映射 0.3–0.75）按 bge-m3 在 `tools/search-eval` 上调过。带 GPU 的二进制约 47 MB，`cargo build --no-default-features` 约 15 MB。bge-m3 质量好且跨语言（英文查询可命中中文段落）。
- 语义精排需要 `--model-dir` 指向 BERT 类嵌入模型（config.json、tokenizer.json、model.safetensors，如 BAAI/bge-small-en-v1.5；可用 `seesea-embed.json` 设置 pooling 和前缀），基于 candle 在 CPU 上推理，无 C/C++ 运行时。不提供模型时退回词法精排（BM25 + 标题支撑度）。余弦到分数的映射（`cos_floor`/`cos_ceil`）按 BGE 类模型设定，尚未在标注集上验证。
- `deep` 模式读取更多页面，但尚未做查询改写。
- SSRF 防护检查目标与重定向，未固定 DNS 解析结果。
- 缓存：查询按有意义的词归一化（大小写、词序、停用词不影响命中）；页面缓存按每个 URL 的变化情况自适应延长或缩短有效期；`--fuzzy-cache 0.8`（默认关）让相近查询复用结果，先用 `seesea replay` 看代价；服务端 `GET /v1/stats` 给出命中率。
- 存储只有内存/文件缓存，尚未采用 Turso。

设计文档：[docs/search-pipeline-design.md](docs/search-pipeline-design.md)。许可证：AGPL-3.0-or-later。
