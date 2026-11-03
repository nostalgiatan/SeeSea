# SeeSea

[English](./README_EN.md) | 中文

SeeSea 是给 AI 用的搜索服务，一个纯 Rust 的单文件程序。一次查询会同时发给多个搜索来源，合并排序，下载排名靠前的页面并抽取正文，按正文重新判断相关性，最后在字符预算内只返回回答问题的章节。

它可以当命令行工具用，也可以作为 HTTP 服务（`/v1/search`、`/v1/extract`）或 MCP 服务运行。需要时可以拆成网关、搜索节点和数据节点组成的集群。

## 快速开始

构建：

```bash
cd seesea-core
cargo build --release -p seesea-cli
```

可执行文件是 `target/release/seesea`。搜索：

```bash
seesea search "rust async runtime"             # 读取排名靠前页面的正文
seesea search -m fast -n 5 --json "query"      # 只合并摘要，几秒内返回
seesea extract https://example.com/post -q "thread pool"
```

作为服务运行：

```bash
seesea serve --addr 127.0.0.1:8080
curl "http://127.0.0.1:8080/v1/search?q=rust+async&mode=fast"
```

不配置任何东西也能搜索：Bing、DuckDuckGo、Mojeek 和 Wikipedia 是内置来源。这些来源常被验证码拦截，想要更稳定，请接入自建的 SearXNG 或 Brave Search，见[配置搜索来源](docs/how-to/configure-sources.md)。

## 它是怎么工作的

查询经过扇出、融合、第一层排序（只看摘要）、抓取并抽取、第二层排序（看正文）和章节选择几级。标题没有独立的权重，只有被正文支撑才算分。每一级都可以缺席：没有模型时用词法排序，来源被拦截时用其余来源，每次降级都会写进响应的 `degraded`。

准备了 bge-m3（嵌入）和 bge-reranker-v2-m3（交叉编码器）两个模型时，排序质量更好。它们在 Vulkan GPU 上运行，没有显卡时自动改用 CPU。见[准备本地模型](docs/how-to/set-up-models.md)。

另有几个可选部分：

- 本地索引：从 RSS、Atom 和推送接口收集新近文章，作为 `local` 来源参与搜索。
- 用户、密钥、套餐和限流：`seesea serve` 自带。
- 多节点：网关、搜索节点、flywheel 数据节点和见证节点，节点间用自定义的 QUIC 协议通信，没有明文传输。

## 文档

文档分成教程、操作指南、参考和解释四类，入口在 [docs/README.md](docs/README.md)。

| 想做的事 | 去哪里 |
|---|---|
| 第一次用 | [教程：第一次搜索](docs/tutorials/first-search.md) |
| 接入 SearXNG 或 Brave | [配置搜索来源](docs/how-to/configure-sources.md) |
| 给 MCP 客户端用 | [接入 MCP 客户端](docs/how-to/connect-an-mcp-client.md) |
| 对外提供服务 | [运行 HTTP 服务并发放密钥](docs/how-to/run-the-server.md) |
| 部署集群 | [部署多节点集群](docs/how-to/deploy-a-cluster.md) |
| 查命令、接口、配置 | [参考](docs/README.md#参考) |
| 理解设计 | [搜索管线如何工作](docs/explanation/pipeline.md) |

## 现状

测试覆盖了管线、HTTP 服务、MCP、账号、集群协议和故障转移。多机部分用 `scripts/netns-cluster.sh` 在一台机器的网络命名空间里验证过，没有在真实多机或跨机房网络上测试。

搜索质量只在一个 8 个查询的小评测集和一次 4 条查询的 Tavily 对比里看过，还不能说明整体水平。[已知限制](docs/reference/known-limits.md)列出了做不到和没有验证的事。

## 目录

| 位置 | 内容 |
|---|---|
| `seesea-core/crates/seesea-cli` | 命令行、HTTP 服务、MCP、多节点和内置来源 |
| `seesea-core/crates/seesea-pipeline` | 搜索管线、排序、学习、搜索记录 |
| `seesea-core/crates/seesea-extract` | 网页正文抽取 |
| `seesea-core/crates/seesea-embed` | 嵌入模型和交叉编码器的推理 |
| `seesea-core/crates/seesea-fetch` | 页面抓取 |
| `seesea-core/crates/seesea-index` | 本地索引和订阅源 |
| `seesea-core/crates/seesea-auth` | 用户、密钥、套餐 |
| `seesea-core/crates/seesea-mcp` | MCP 工具定义 |
| `seesea-core/scripts` | 多节点测试和 Tavily 对比脚本 |
| `tools` | 评测和自动检查工具 |

运行全部测试：

```bash
cd seesea-core
cargo test --workspace
```

## 许可证

AGPL-3.0-or-later。
