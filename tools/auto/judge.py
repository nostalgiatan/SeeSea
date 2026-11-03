#!/usr/bin/env python3
"""Turn graded candidates into judgments for `seesea eval`.

  seesea report            # writes candidates.jsonl: each query with its top results and "grade": null
  (fill in "grade": 0-3 per candidate: 3 exactly what was wanted, 2 answers it, 1 related, 0 not relevant)
  tools/auto/judge.py candidates.jsonl >> tools/search-eval/live-judgments.jsonl

Queries with no graded candidate are skipped. Nothing is invented: ungraded results are left out, which
`seesea eval` counts as not relevant, so grade every candidate you want counted as wrong with 0 only if you like;
leaving it null has the same effect.
"""
import json, sys
for line in open(sys.argv[1]):
    line = line.strip()
    if not line:
        continue
    c = json.loads(line)
    rel = [{"url": x["url"], "grade": int(x["grade"])} for x in c["candidates"] if x.get("grade") not in (None, 0)]
    if rel:
        print(json.dumps({"query": c["query"], "relevant": rel}, ensure_ascii=False))
