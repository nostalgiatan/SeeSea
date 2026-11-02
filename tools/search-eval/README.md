# Offline search-quality set

Eight queries (English, Chinese, cross-lingual, vocabulary-mismatch) over a local fake web: each has an
answer page (grade 2), a partial page (grade 1), a bait page whose title echoes the query over spam, a
keyword-stuffed page and other topics' answers as distractors. The fake SearXNG returns them in a bad
order on purpose. It is small and synthetic: use it to catch regressions and to choose between settings,
not as proof of quality on the open web.

```bash
python3 tools/search-eval/corpus.py &        # serves http://127.0.0.1:18765
seesea --searxng http://127.0.0.1:18765 --no-native --no-cache --allow-private \
  [--model-dir bge-m3-q4_0.gguf] [--rerank-model bge-reranker-v2-m3-q4_k_m.gguf] eval tools/search-eval/judgments.jsonl -k 5 --mode deep   # fast mode never fetches or reranks
```

Mean nDCG@5 on this set (`--mode deep`): lexical only 0.828; bge-m3 (Q4_0) 0.952; bge-reranker-v2-m3 alone 0.945; both 0.970 (cross weight 0.35 to 0.5 tie; 0.65 and up drops to 0.945). Before the anti-stuffing factor these were 0.645, 0.874 and 0.867. Pages are short, so the number of passages scored changes nothing here.
