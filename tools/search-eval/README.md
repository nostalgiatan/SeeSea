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

Thirty-seven queries now: the original eight plus 29 in `topics_extra.py`, each with an answer (grade 2), a partial page (grade 1)
and a hard distractor that shares the query's words but answers something else (grade 0), e.g. "TCP and UDP: a band review".
I wrote and graded them by hand; the live web was not reachable from the build machine, so this stays synthetic.

Mean nDCG@5 (`--mode deep`, 37 queries): lexical only 0.701; bge-m3 0.812; reranker alone 0.781; both with the old final
weights (0.4 snippet-level, 0.5 relevance, 0.1 body quality) 0.832, both with 0.2/0.7/0.1 0.859 and recall 1.0. The snippet-level
score is what bait and distractor titles game, so it now counts less once the page has been read. Giving body quality more weight
scored higher still (0.89), but the distractors here are short by construction, so that gain mostly rewards page length and was not taken.
