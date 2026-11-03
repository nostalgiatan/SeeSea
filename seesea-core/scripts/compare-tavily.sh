#!/usr/bin/env bash
# Run the same queries through Tavily and seesea; write one JSON file per query and engine.
# usage: TAVILY_API_KEY=tvly-... scripts/compare-tavily.sh QUERIES.txt OUTDIR [seesea args...]
# QUERIES.txt: one query per line. Extra args go to `seesea search`; models come from the default models directory.
set -u
Q=${1:?queries file}; OUT=${2:?output dir}; shift 2
: "${TAVILY_API_KEY:?set TAVILY_API_KEY}"
BIN=${SEESEA_BIN:-seesea}
if ! "$BIN" models 2>&1 | tee /dev/stderr | grep -qi "valid\|ok\|ready"; then
  echo "seesea models shows no usable model; fix that first" >&2; exit 1
fi
mkdir -p "$OUT/tavily" "$OUT/seesea"
i=0
while IFS= read -r q; do
  [ -z "$q" ] && continue
  i=$((i+1)); n=$(printf %03d $i)
  echo "$q" > "$OUT/tavily/$n.query"
  body=$(python3 -c 'import json,sys;print(json.dumps({"query":sys.argv[1],"search_depth":"advanced","max_results":10,"include_raw_content":"text","chunks_per_source":3}))' "$q")
  curl -sS --max-time 90 https://api.tavily.com/search \
    -H "Authorization: Bearer $TAVILY_API_KEY" -H 'Content-Type: application/json' \
    -d "$body" > "$OUT/tavily/$n.json"
  "$BIN" search --json -m standard -n 10 "$@" -- "$q" > "$OUT/seesea/$n.json" 2> "$OUT/seesea/$n.err"
  echo "[$n] $q"
done < "$Q"
echo "done: $OUT"
