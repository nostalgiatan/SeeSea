# 与 Tavily 对比搜索结果

本页说明如何把同一批查询分别交给 Tavily 和 seesea，逐条比较结果。你需要一个 Tavily API 密钥，并且 seesea 所在机器能联网。Tavily 的排序只能当参照，不是标准答案。

## 准备

1. 确认两个模型都可用。脚本会先运行 `seesea models`，嵌入模型或重排模型缺失时直接退出，不会悄悄退回词法排序。设置方法见[准备本地模型](set-up-models.md)。
2. 写一个查询文件，每行一条。建议 20 条以上，覆盖中英文、事实类、新闻类、技术问答和长尾问题：

   ```bash
   cat > queries.txt <<'EOF'
   rust tokio select 取消安全
   what is the latest stable version of PostgreSQL
   2026 年 新能源汽车 销量 排名
   EOF
   ```

## 运行

```bash
TAVILY_API_KEY=tvly-... SEESEA_BIN=./target/release/seesea \
  scripts/compare-tavily.sh queries.txt out
```

脚本对每条查询：

- 调用 Tavily（`search_depth` 为 `advanced`，10 条结果，带正文）；
- 用 `seesea search --json -m standard -n 10` 搜索；
- 把结果存到 `out/tavily/NNN.json` 和 `out/seesea/NNN.json`，查询词存到 `out/tavily/NNN.query`，seesea 的标准错误存到 `out/seesea/NNN.err`。

脚本在 `--` 之后传查询词，额外的参数会原样交给 `seesea search`，可以附加 `--lang`、`--include` 等。

## 读结果

先看三项：

| 看什么 | 在哪里 |
|---|---|
| 两边前十个网址的重合，以及 Tavily 的高分页面在 seesea 里排第几 | 两边的 `hits`/`results` 数组 |
| seesea 的页面有没有读到正文，读不到的有多少 | `hits[].content`、`hits[].content_error`、`degraded` |
| 时间花在哪里 | `stages`，见[搜索响应参考](../reference/search-response.md#stages) |

重合度低不等于差。两边的搜索引擎、新鲜度和抽取方式都不同，需要逐条看内容才能判断哪边的结果更能回答问题。

## 解读时的注意

- 同一批查询要一次跑完，网页内容和排名会随时间变化。
- 4 条查询不足以下结论。需要带人工标注的更大数据集，才能量化跑题率。
- seesea 的耗时包含下载页面，通常比只返回摘要的 API 长很多。比较速度时用 `-m fast` 另跑一遍。
