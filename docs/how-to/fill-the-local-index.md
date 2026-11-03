# 填充本地索引

本地索引保存新近文章的标题、约 400 字的导语、日期和主机，不保存正文。每次搜索都会把它当作 `local` 来源查询，所以有索引时，即使网络很慢也能找到近期内容。索引为什么这样设计，见[新鲜度与本地索引](../explanation/freshness-and-index.md)。

## 订阅 RSS 和 Atom

加入内置的中英文起始订阅源：

```bash
seesea feeds seed
```

这些源先进入试用期，坏的会自动停用。有些地址在你的网络里可能连不上，用 `seesea feeds list` 看每个源最近一次的错误。

加入你自己的订阅源：

```bash
seesea feeds add https://example.org/feed.xml
```

让 seesea 找出一个站点提供的订阅源：

```bash
seesea feeds discover example.org --dry-run   # 只列出
seesea feeds discover example.org             # 登记
```

拉取一次，或持续拉取：

```bash
seesea poll
seesea poll --every 15m
```

`seesea serve` 默认每 15 分钟在后台拉取，所以运行服务时不需要另外开 `poll`。

处理有问题的订阅源：

| 目的 | 命令 |
|---|---|
| 看状态 | `seesea feeds list` |
| 移除 | `seesea feeds remove <网址>` |
| 重新启用被停用的 | `seesea feeds revive <网址>` |

## 主动推送文章

向运行中的服务推送文章，需要根密钥，每次最多 1000 条：

```bash
curl -X POST http://127.0.0.1:8080/v1/ingest \
  -H "Authorization: Bearer $SEESEA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"items":[{"url":"https://example.org/a","title":"标题","lead":"导语","published":1767225600,"source":"my-pusher"}]}'
```

## 生成向量，启用语义检索

索引条目有了向量之后，英文问题也能找到中文报道。轮询每轮为 256 条条目生成向量。想立刻补齐：

```bash
seesea index embed
```

查看某个问题在向量空间里最近的条目和距离（位）：

```bash
seesea index near "refrigerators hacked in supermarkets" -k 5
```

这需要嵌入模型，见[准备本地模型](set-up-models.md)。

## 查找相似页面

服务运行时，按含义找与某个页面或某段文字相近的内容：

```bash
curl -X POST http://127.0.0.1:8080/v1/similar \
  -H "Content-Type: application/json" \
  -d '{"url":"https://example.org/a","limit":5}'
```

## 检查索引的大小，应用保留策略

```bash
seesea index          # 条数、天数、体积
seesea index prune    # 立即按保留策略清理
```

默认保留 30 天，总量上限 512 MB，超出时先删最旧的一天。在配置文件里用 `retention_days` 和 `index_quota_mb` 调整。

## 接收 WebSub 推送

订阅源声明了 WebSub hub 时，给服务一个 hub 能访问的地址：

```bash
seesea serve --public-url https://seesea.example.com
```

服务会向 hub 订阅，回调在 `/v1/websub/{id}`，到期前自动续订。轮询仍作为兜底。
