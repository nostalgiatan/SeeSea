# 检查搜索质量

本页说明如何用搜索记录、标注数据和反馈发现并衡量排序问题。和另一个搜索服务对比的做法见[与 Tavily 对比](compare-with-tavily.md)。

## 查看最近的搜索有什么问题

每次搜索（命令行、HTTP、MCP）和每条反馈都会记入数据目录下的 `journal/`，每天一个 JSONL 文件，保留 60 天。记录里有查询、各结果的排名和分数构成、来源失败原因、各阶段耗时和读不了的页面。`--no-journal` 或配置里的 `no_journal = true` 可以关闭。

汇总最近 7 天：

```bash
seesea report --since 7d
```

它写出四个文件，默认放在数据目录下的新文件夹里，用 `--out` 指定位置：

| 文件 | 内容 |
|---|---|
| `report.md`、`report.json` | 汇总：失败的来源、空结果、读不了的主机 |
| `suspects.jsonl` | 看起来有问题的搜索：无结果、来源全败、顶部分数低、页面多数读不了、过慢 |
| `candidates.jsonl` | 每个查询的前十个结果，`grade` 为空，等你填 |

## 标注并评分

1. 打开 `candidates.jsonl`，给每个结果的 `grade` 填 0 到 3。
2. 把它转成评测文件：

   ```bash
   tools/auto/judge.py candidates.jsonl >> tools/search-eval/live-judgments.jsonl
   ```

3. 用评测文件给当前配置打分：

   ```bash
   seesea eval tools/search-eval/live-judgments.jsonl -k 10 -m standard
   ```

   输出 nDCG、召回和 MRR。

评测文件的每一行是一个查询和它的相关网址（带等级）。

## 一次跑完构建、测试和评测

```bash
tools/auto/run.sh
```

它构建、测试、做离线评测、探测来源、出报告，全部落在 `runs/<时间>/`，并与上一次对比。

## 告诉排序某个结果好还是坏

```bash
seesea feedback https://example.org/a bad
```

HTTP 接口是 `POST /v1/feedback`。一次反馈相当于第一层排序里 5 次观察。

## 查看第一层学到了什么

```bash
seesea learn show --top 15    # 表现最好和最差的主机，以及每个来源
seesea learn reset            # 清空
```

不想让第一层学习，加 `--no-learn`；`eval` 本身不学习。

## 用日志评估缓存策略

```bash
seesea search --log s.jsonl "query"          # 多跑几次不同的查询
seesea replay s.jsonl --ttl 600              # 命中率，以及缓存结果与真实结果的差异
```

`replay` 默认比较模糊阈值 0.9、0.8、0.7。开启 `--fuzzy-cache` 之前先看它给出的代价。
