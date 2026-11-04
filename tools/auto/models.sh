# Sourced by run.sh and try.sh: finds the two model files so nothing is downloaded when they are already here.
# Sets EMB and RER (empty when not found). Override with SEESEA_EMBED_FILE / SEESEA_RERANK_FILE.
find_model() { # $1 = name pattern, case-insensitive
  local d
  for d in "${SEESEA_MODELS_DIR:-}" "${XDG_DATA_HOME:-$HOME/.local/share}/seesea/models" "$ROOT" "$ROOT/.." \
           "$(git -C "$ROOT" rev-parse --git-common-dir 2>/dev/null | xargs -I{} dirname {} 2>/dev/null)" "$HOME/project/SeeSea" "$PWD"; do
    [ -n "$d" ] && [ -d "$d" ] || continue
    f=$(find "$d" -maxdepth 1 -iname "$1" -type f 2>/dev/null | head -1)
    [ -n "$f" ] && { echo "$f"; return; }
  done
}
EMB="${SEESEA_EMBED_FILE:-$(find_model 'bge-m3-q4_0.gguf')}"
RER="${SEESEA_RERANK_FILE:-$(find_model 'bge-reranker-v2-m3-q4_k_m.gguf')}"
MODEL_FLAGS=()
[ -n "$EMB" ] && MODEL_FLAGS+=(--model-dir "$EMB")
[ -n "$RER" ] && MODEL_FLAGS+=(--rerank-model "$RER")
