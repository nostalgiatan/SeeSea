# Compare with Tavily

This page shows how to give the same queries to Tavily and to seesea and compare the results one by one. You need a Tavily API key, and the seesea machine needs internet access. Tavily's ranking is a reference, not the right answer.

## Prepare

1. Make sure both models are available. The script first runs `seesea models` and exits if the embedding or reranker model is missing, so it never falls back to word-based ranking without saying so. See [Set up local models](set-up-models.md).
2. Write a query file, one query per line. Use 20 or more, covering Chinese and English, facts, news, technical questions and long-tail queries:

   ```bash
   cat > queries.txt <<'EOF'
   rust tokio select 取消安全
   what is the latest stable version of PostgreSQL
   2026 年 新能源汽车 销量 排名
   EOF
   ```

## Run

```bash
TAVILY_API_KEY=tvly-... SEESEA_BIN=./target/release/seesea \
  scripts/compare-tavily.sh queries.txt out
```

For each query the script:

- calls Tavily (`search_depth` set to `advanced`, 10 results, with text);
- searches with `seesea search --json -m standard -n 10`;
- saves the results as `out/tavily/NNN.json` and `out/seesea/NNN.json`, the query as `out/tavily/NNN.query`, and seesea's standard error as `out/seesea/NNN.err`.

The script passes the query after `--`, and any extra arguments go to `seesea search` unchanged, so you can add `--lang`, `--include` and so on.

## Read the results

Look at three things first:

| What | Where |
|---|---|
| How much the top ten URLs overlap, and where Tavily's high-scoring pages rank in seesea | The `hits` and `results` arrays on both sides |
| Whether seesea read the pages, and how many it could not | `hits[].content`, `hits[].content_error`, `degraded` |
| Where the time went | `stages`; see the [search response reference](../reference/search-response.md#stages) |

Low overlap does not mean worse. The two use different engines, freshness and extraction, and you have to read the content to say which side answers the question better.

## Keep in mind

- Run the whole set at once. Page content and rankings change over time.
- Four queries are too few to conclude anything. You need a larger judged set to put a number on off-topic results.
- seesea's time includes downloading pages, so it is usually much longer than an API that returns snippets. To compare speed, run again with `-m fast`.
