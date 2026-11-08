# 文档

中文 | [English](en/README.md)

文档按用途分成四类。找不到要的内容时，先想清楚你是想学、想做、想查，还是想懂。

## 教程

跟着做一遍，学会基本用法。

- [第一次搜索](tutorials/first-search.md)

## 操作指南

已经知道目标，需要一组步骤。

搜索：

- [配置搜索来源](how-to/configure-sources.md)
- [准备本地模型](how-to/set-up-models.md)
- [填充本地索引](how-to/fill-the-local-index.md)
- [检查搜索质量](how-to/review-search-quality.md)
- [把钓鱼站挡在结果之外](how-to/keep-phishing-out.md)
- [与 Tavily 对比](how-to/compare-with-tavily.md)

服务与接入：

- [运行 HTTP 服务并发放密钥](how-to/run-the-server.md)
- [接入 MCP 客户端](how-to/connect-an-mcp-client.md)
- [使用工作台](how-to/use-the-workbench.md)
- [使用桌面版](how-to/use-the-desktop-app.md)

多节点：

- [部署多节点集群](how-to/deploy-a-cluster.md)
- [把单节点的数据迁到集群](how-to/move-data-to-a-cluster.md)
- [在一台机器上测试集群](how-to/test-a-cluster-locally.md)

## 参考

查一个具体的参数、字段或取值。

- [命令行](reference/cli.md)
- [工作台](reference/workbench.md)
- [HTTP 接口](reference/http-api.md)
- [MCP 工具](reference/mcp.md)
- [配置文件](reference/config.md)
- [搜索响应](reference/search-response.md)
- [节点间协议 seesea-wire/1](reference/wire-protocol.md)
- [已知限制](reference/known-limits.md)

## 解释

理解为什么这样设计。

- [搜索管线如何工作](explanation/pipeline.md)
- [排序与学习](explanation/ranking-and-learning.md)
- [结果可信度判断](explanation/vet.md)
- [高并发下的模型负载](explanation/model-load.md)
- [新鲜度与本地索引](explanation/freshness-and-index.md)
- [多节点设计](explanation/cluster-design.md)
- [正文抽取研究](explanation/extraction-research.md)

文档写作遵循 [Diátaxis](https://diataxis.fr) 框架。
