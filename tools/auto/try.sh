#!/usr/bin/env bash
# Exercises what only the real web can show: write-back, the local index, similar-page search, batch search.
# Everything it prints and every file it makes goes to runs/try-<time>/ ; send that folder (or its path) back.
# Usage: tools/auto/try.sh [--no-models]
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT/seesea-core"
OUT="$ROOT/runs/try-$(date +%Y%m%d-%H%M%S)"; mkdir -p "$OUT"
export CARGO_PROFILE_DEV_DEBUG=0
cargo build -p seesea-cli >"$OUT/build.txt" 2>&1 || { echo "build failed: $OUT/build.txt"; exit 1; }
B="$ROOT/seesea-core/target/debug/seesea"
MODELS=1; [ "${1:-}" = "--no-models" ] && MODELS=0
. "$ROOT/tools/auto/models.sh"
if [ $MODELS = 1 ]; then
  if [ -n "$EMB" ] && [ -n "$RER" ]; then
    echo "models found: $EMB $RER" | tee "$OUT/models.txt"
  else
    echo "models not found locally; downloading (set SEESEA_EMBED_FILE / SEESEA_RERANK_FILE to point at them instead)"
    b models download >"$OUT/models.txt" 2>&1 && MODEL_FLAGS=() || { echo "download failed (see models.txt); continuing without"; MODELS=0; MODEL_FLAGS=(); }
  fi
else
  MODEL_FLAGS=()
fi
B_ORIG="$B"
b() { "$B_ORIG" ${MODEL_FLAGS[@]+"${MODEL_FLAGS[@]}"} "$@"; }

step() { echo "== $*"; }
{
step "sources: which engines work from here"
b sources --check

step "searches (standard mode reads pages; good ones are written back to the index)"
for q in "why does python have a global interpreter lock" "postgres vacuum full vs autovacuum" \
         "红烧肉怎么做才软糯" "difference between tcp and udp" "tokio scheduler work stealing" \
         "rust borrow checker cannot borrow as mutable more than once"; do
  echo "--- $q"
  b search -m standard -n 5 "$q" 2>&1 | grep -E "^## |^http|^score|^error|returned nothing|failed" | head -15
done

step "index after those searches"
b index
[ $MODELS = 1 ] && { step "embed the new items"; b index embed; b index; }

step "same topics, reworded, web switched off by a dead proxy: only the index can answer"
for q in "gil python reason" "vacuum full or autovacuum postgres" "tcp versus udp"; do
  echo "--- $q"
  b --proxy http://127.0.0.1:9 search -m fast -n 3 "$q" | grep -E "^## |^http|found by|returned|failed" | head -8
done
[ $MODELS = 1 ] && { step "nearest items by meaning"; b index near "how does python handle threads" -k 5; }

step "http: batch and similar"
PORT=18899
b serve --addr 127.0.0.1:$PORT --allow-open --poll-every 0s >"$OUT/serve.log" 2>&1 &
SRV=$!
for _ in $(seq 1 60); do curl -sf "http://127.0.0.1:$PORT/health" >/dev/null && break; sleep 1; done
curl -s -X POST "http://127.0.0.1:$PORT/v1/search/batch" -H 'content-type: application/json' \
  -d '{"queries":["rust async runtime","docker container exits immediately"],"mode":"fast","limit":3}' \
  | python3 -c "import json,sys; d=json.load(sys.stdin); [print(r['query'], 'ok' if r['ok'] else r['error']['code'], [h['url'] for h in r.get('response',{}).get('hits',[])]) for r in d.get('results',[])] if 'results' in d else print(d)"
curl -s -X POST "http://127.0.0.1:$PORT/v1/similar" -H 'content-type: application/json' \
  -d '{"text":"how does the python interpreter handle threads","limit":5}'
echo
step "domain filter and news topic"
curl -s "http://127.0.0.1:$PORT/v1/search?q=rust+release&mode=fast&include_domains=github.com&limit=3" | python3 -c "import json,sys; print([h['url'] for h in json.load(sys.stdin).get('hits',[])])"
curl -s "http://127.0.0.1:$PORT/v1/search?q=ai+regulation&mode=fast&topic=news&limit=5" | python3 -c "import json,sys; d=json.load(sys.stdin); print([(h['url'],h['published']) for h in d.get('hits',[])] or d)"
kill $SRV 2>/dev/null
} >"$OUT/try.txt" 2>&1

b report --since 1h --out "$OUT/report" >/dev/null 2>&1
echo "done. Send me: $OUT"
echo "  try.txt (what happened), report/ (suspects, candidates), serve.log"
