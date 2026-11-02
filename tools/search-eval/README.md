# Offline search-quality set

Eight queries (English, Chinese, cross-lingual, vocabulary-mismatch) over a local fake web: each has an
answer page (grade 2), a partial page (grade 1), a bait page whose title echoes the query over spam, a
keyword-stuffed page and other topics' answers as distractors. The fake SearXNG returns them in a bad
order on purpose. It is small and synthetic: use it to catch regressions and to choose between settings,
not as proof of quality on the open web.

```bash
python3 tools/search-eval/corpus.py &        # serves http://127.0.0.1:18765
seesea --searxng http://127.0.0.1:18765 --no-native --no-cache --allow-private \
  [--model-dir bge-m3-q4_0.gguf] eval tools/search-eval/judgments.jsonl -k 5
```

Mean nDCG@5 on this set: lexical only 0.645; bge-m3 (Q4_0) with weight 0.6 0.825, 0.8 0.874.
