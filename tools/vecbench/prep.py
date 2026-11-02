"""Build a retrieval test set from WebMainBench: docs.jsonl and queries.jsonl.

Each document is indexed by its opening ("lead", the first LEAD characters of the main content,
title not included). Two kinds of query, both known-item (the page the query came from is the
only right answer):
  title    the page title
  sentence a sentence from further down the page, beyond the lead
Usage: prep.py WEBMAINBENCH.jsonl OUT_DIR [N_DOCS] [N_QUERIES_PER_KIND]
"""
import json, re, random, sys, html

LEAD = 800
src, out = sys.argv[1], sys.argv[2]
n_docs = int(sys.argv[3]) if len(sys.argv) > 3 else 5000
n_q = int(sys.argv[4]) if len(sys.argv) > 4 else 1200
random.seed(7)

def clean(md):
    md = re.sub(r'!\[[^\]]*\]\([^)]*\)', ' ', md)
    md = re.sub(r'\[([^\]]*)\]\([^)]*\)', r'\1', md)
    md = re.sub(r'[#*_`>|]+', ' ', md)
    return re.sub(r'\s+', ' ', md).strip()

docs = []
with open(src) as f:
    for line in f:
        d = json.loads(line)
        m = re.search(r'<title[^>]*>(.*?)</title>', d['html'], re.S | re.I)
        title = html.unescape(re.sub(r'\s+', ' ', m.group(1))).strip() if m else ''
        text = clean(d.get('convert_main_content') or '')
        if len(title) < 15 or len(text) < LEAD + 700:
            continue
        docs.append({'id': len(docs), 'lang': d['meta'].get('language', '?'), 'title': title,
                     'lead': text[:LEAD], 'rest': text[LEAD + 100:]})
        if len(docs) >= n_docs:
            break

def sentence(rest):
    cands = [s.strip() for s in re.split(r'(?<=[.!?。！？])\s+', rest) if 70 <= len(s.strip()) <= 220
             and 'http' not in s and s.count(' ') > 6]
    return random.choice(cands) if cands else None

picked = random.sample(docs, min(n_q, len(docs)))
queries = []
for d in picked:
    queries.append({'kind': 'title', 'text': d['title'], 'target': d['id']})
    s = sentence(d['rest'])
    if s:
        queries.append({'kind': 'sentence', 'text': s, 'target': d['id']})
with open(f'{out}/docs.jsonl', 'w') as f:
    for d in docs:
        f.write(json.dumps({'id': d['id'], 'lead': d['lead'], 'lang': d['lang']}, ensure_ascii=False) + '\n')
with open(f'{out}/queries.jsonl', 'w') as f:
    for q in queries:
        f.write(json.dumps(q, ensure_ascii=False) + '\n')
print(len(docs), 'docs,', len(queries), 'queries')
