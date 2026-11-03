#!/usr/bin/env bash
# One command that leaves everything needed to look at the project's state in runs/<timestamp>/:
#   tests.txt          workspace test results
#   eval-*.txt         offline search-quality scores (lexical; with models when they are installed)
#   eval-live.txt      the same for tools/search-eval/live-judgments.jsonl, when that file exists
#   sources.txt        a live probe of every search source from this machine
#   report/            `seesea report` over the recorded searches (report.md, suspects.jsonl, candidates.jsonl)
#   summary.md/json    the numbers in one place, with the change since the previous run
# Usage: tools/auto/run.sh [--skip-tests] [--no-live]
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT/seesea-core"
OUT="$ROOT/runs/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$OUT"
SKIP_TESTS=0; LIVE=1
for a in "$@"; do case "$a" in --skip-tests) SKIP_TESTS=1;; --no-live) LIVE=0;; esac; done

export CARGO_PROFILE_DEV_DEBUG=0
cargo build -p seesea-cli >"$OUT/build.txt" 2>&1 || { echo "build failed, see $OUT/build.txt"; exit 1; }
BIN="$ROOT/seesea-core/target/debug/seesea"
EVAL=(--no-native --no-cache --allow-private --no-index --no-learn --no-journal)

if [ $SKIP_TESTS = 0 ]; then
  cargo test --workspace >"$OUT/tests.txt" 2>&1
  echo "tests: $(grep -c '^test .* ok$' "$OUT/tests.txt") passed, $(grep -c '^test .* FAILED$' "$OUT/tests.txt") failed"
fi

# Offline set: a fake web served locally.
python3 "$ROOT/tools/search-eval/corpus.py" & CORPUS=$!
sleep 1
MODELS="${SEESEA_MODEL_DIR_BASE:-${XDG_DATA_HOME:-$HOME/.local/share}/seesea/models}"
EMB="$MODELS/bge-m3-Q4_0.gguf"; RER="$MODELS/bge-reranker-v2-m3-Q4_K_M.gguf"
J="$ROOT/tools/search-eval/judgments.jsonl"
"$BIN" --searxng http://127.0.0.1:18765 "${EVAL[@]}" eval "$J" -k 5 --mode deep >"$OUT/eval-lexical.txt" 2>&1
if [ -f "$EMB" ] && [ -f "$RER" ]; then
  "$BIN" --searxng http://127.0.0.1:18765 "${EVAL[@]}" --model-dir "$EMB" --rerank-model "$RER" eval "$J" -k 5 --mode deep >"$OUT/eval-models.txt" 2>&1
fi
kill $CORPUS 2>/dev/null

# Judgments made from real searches (see tools/auto/judge.py), run against the live web.
LIVEJ="$ROOT/tools/search-eval/live-judgments.jsonl"
if [ $LIVE = 1 ]; then
  if [ -f "$LIVEJ" ]; then
    "$BIN" --no-journal --no-learn eval "$LIVEJ" -k 10 --mode standard >"$OUT/eval-live.txt" 2>&1
  fi
  "$BIN" sources --check >"$OUT/sources.txt" 2>&1
fi

"$BIN" report --since 7d --out "$OUT/report" >/dev/null 2>&1

python3 - "$OUT" "$ROOT/runs" <<'PY'
import json, os, re, sys
out, runs = sys.argv[1], sys.argv[2]
def last(path):
    try:
        for l in reversed(open(path).read().splitlines()):
            m = re.search(r"mean ndcg@\d+ ([\d.]+)\s+recall ([\d.]+)\s+mrr ([\d.]+)\s+\((\d+) queries, (\d+) failed", l)
            if m: return dict(ndcg=float(m[1]), recall=float(m[2]), mrr=float(m[3]), queries=int(m[4]), failed=int(m[5]))
    except OSError: pass
s = {}
for k in ("lexical", "models", "live"):
    r = last(f"{out}/eval-{k}.txt")
    if r: s["eval_" + k] = r
t = open(f"{out}/tests.txt").read() if os.path.exists(f"{out}/tests.txt") else ""
if t:
    s["tests"] = dict(passed=len(re.findall(r"^test .* ok$", t, re.M)), failed=len(re.findall(r"^test .* FAILED$", t, re.M)))
rep = f"{out}/report/report.json"
if os.path.exists(rep):
    r = json.load(open(rep)); s["journal"] = dict(searches=r["searches"], suspects=len(r["suspects"]), errors=sum(r["errors"].values()))
prev = sorted(d for d in os.listdir(runs) if os.path.exists(f"{runs}/{d}/summary.json") and f"{runs}/{d}" != out) if os.path.isdir(runs) else []
p = json.load(open(f"{runs}/{prev[-1]}/summary.json")) if prev else {}
json.dump(s, open(f"{out}/summary.json", "w"), indent=2)
lines = ["# Run " + os.path.basename(out), ""]
for k, v in s.items():
    old = p.get(k, {})
    delta = ""
    if "ndcg" in v and "ndcg" in old: delta = f"  (was {old['ndcg']:.3f}, {v['ndcg']-old['ndcg']:+.3f})"
    lines.append(f"- {k}: " + ", ".join(f"{a} {b}" for a, b in v.items()) + delta)
open(f"{out}/summary.md", "w").write("\n".join(lines) + "\n")
print("\n".join(lines)); print("\nFiles in", out)
PY
