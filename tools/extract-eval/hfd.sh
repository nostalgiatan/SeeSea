#!/usr/bin/env bash
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; DIM='\033[2m'; BOLD='\033[1m'; NC='\033[0m' # No Color

trap 'printf "\n%bInterrupted. Re-run to resume.%b\n" "$YELLOW" "$NC"; exit 130' INT

# Format a byte count as a human-readable string (decimal units, matching the HF UI).
human() {
    awk -v b="${1:-0}" 'BEGIN{u="B KB MB GB TB PB";n=split(u,a," ");i=1;
        while(b>=1000&&i<n){b/=1000;i++} printf (i==1?"%d%s":"%.2f%s"),b,a[i]}'
}

display_help() {
    cat << EOF
Usage:
  hfd <REPO_ID> [--include include_pattern1 include_pattern2 ...] [--exclude exclude_pattern1 exclude_pattern2 ...] [--hf_username username] [--hf_token token] [--tool aria2c|wget] [-x threads] [-j jobs] [--dataset] [--local-dir path] [--revision rev]

Description:
  Downloads a model or dataset from Hugging Face using the provided repo ID.

Arguments:
  REPO_ID         The Hugging Face repo ID (Required)
                  Format: 'org_name/repo_name' or legacy format (e.g., gpt2)
Options:
  include/exclude_pattern The patterns to match against file path, supports wildcard characters.
                  e.g., '--exclude *.safetensor *.md', '--include vae/*'.
  --include       (Optional) Patterns to include files for downloading (supports multiple patterns).
  --exclude       (Optional) Patterns to exclude files from downloading (supports multiple patterns).
  --hf_username   (Optional) Hugging Face username for authentication (not email).
  --hf_token      (Optional) Hugging Face token for authentication.
  --tool          (Optional) Download tool to use: aria2c (default) or wget.
  -x              (Optional) Number of download threads for aria2c (default: 4).
  -j              (Optional) Number of concurrent downloads for aria2c (default: 5).
  --dataset       (Optional) Flag to indicate downloading a dataset.
  --local-dir     (Optional) Directory path to store the downloaded data.
                             Defaults to the current directory with a subdirectory named 'repo_name'
                             if REPO_ID is composed of 'org_name/repo_name'.
  --revision      (Optional) Model/Dataset revision to download (default: main).

Example:
  hfd gpt2
  hfd bigscience/bloom-560m --exclude *.safetensors
  hfd meta-llama/Llama-2-7b --hf_username myuser --hf_token mytoken -x 4
  hfd lavita/medical-qa-shared-task-v1-toy --dataset
  hfd bartowski/Phi-3.5-mini-instruct-exl2 --revision 5_0
EOF
    exit 1
}

[[ -z "$1" || "$1" =~ ^-h || "$1" =~ ^--help ]] && display_help

REPO_ID=$1
shift

# Default values
TOOL="aria2c"
THREADS=4
CONCURRENT=5
HF_ENDPOINT=${HF_ENDPOINT:-"https://huggingface.co"}
INCLUDE_PATTERNS=()
EXCLUDE_PATTERNS=()
REVISION="main"

validate_number() {
    [[ "$2" =~ ^[1-9][0-9]*$ && "$2" -le "$3" ]] || { printf "%b[Error] %s must be 1-%s%b\n" "$RED" "$1" "$3" "$NC"; exit 1; }
}

# Argument parsing
while [[ $# -gt 0 ]]; do
    case $1 in
        --include) shift; while [[ $# -gt 0 && ! ($1 =~ ^--) && ! ($1 =~ ^-[^-]) ]]; do INCLUDE_PATTERNS+=("$1"); shift; done ;;
        --exclude) shift; while [[ $# -gt 0 && ! ($1 =~ ^--) && ! ($1 =~ ^-[^-]) ]]; do EXCLUDE_PATTERNS+=("$1"); shift; done ;;
        --hf_username) HF_USERNAME="$2"; shift 2 ;;
        --hf_token) HF_TOKEN="$2"; shift 2 ;;
        --tool)
            [[ "$2" == aria2c || "$2" == wget ]] || { printf "%b[Error] Invalid tool. Use 'aria2c' or 'wget'.%b\n" "$RED" "$NC"; exit 1; }
            TOOL="$2"; shift 2 ;;
        -x) validate_number "threads (-x)" "$2" 10; THREADS="$2"; shift 2 ;;
        -j) validate_number "concurrent downloads (-j)" "$2" 10; CONCURRENT="$2"; shift 2 ;;
        --dataset) DATASET=1; shift ;;
        --local-dir) LOCAL_DIR="$2"; shift 2 ;;
        --revision) REVISION="$2"; shift 2 ;;
        *) display_help ;;
    esac
done

# A fingerprint of the options that affect the file list; a change forces regeneration.
generate_command_string() {
    printf 'REPO_ID=%s TOOL=%s INCLUDE=%s EXCLUDE=%s DATASET=%s HF_USERNAME=%s HF_TOKEN=%s HF_ENDPOINT=%s REVISION=%s' \
        "$REPO_ID" "$TOOL" "${INCLUDE_PATTERNS[*]}" "${EXCLUDE_PATTERNS[*]}" "${DATASET:-0}" \
        "${HF_USERNAME:-}" "${HF_TOKEN:-}" "${HF_ENDPOINT:-}" "$REVISION"
}

check_command() {
    if ! command -v "$1" &>/dev/null; then
        printf "%b%s is not installed. Please install it first.%b\n" "$RED" "$1" "$NC"
        exit 1
    fi
}

check_command curl; check_command "$TOOL"

LOCAL_DIR="${LOCAL_DIR:-${REPO_ID#*/}}"
mkdir -p "$LOCAL_DIR/.hfd"

REPO_API_PATH="models/$REPO_ID"; DOWNLOAD_API_PATH="$REPO_ID"
[[ "$DATASET" == 1 ]] && { REPO_API_PATH="datasets/$REPO_ID"; DOWNLOAD_API_PATH=$REPO_API_PATH; }

# wget --cut-dirs strips "<download_api_path>/resolve/<revision>/"; a fixed value wrongly
# assumes the org/name form and eats a directory level for legacy (single-name) repo ids.
CUT_DIRS=$(( $(printf '%s' "$DOWNLOAD_API_PATH" | tr -cd '/' | wc -c) + 3 ))

# Metadata API URL (used for the gated/auth check); append revision when not main.
METADATA_API_PATH="$REPO_API_PATH"
[[ "$REVISION" != "main" ]] && METADATA_API_PATH="$METADATA_API_PATH/revision/$REVISION"
API_URL="$HF_ENDPOINT/api/$METADATA_API_PATH?blobs=true"

METADATA_FILE="$LOCAL_DIR/.hfd/repo_metadata.json"

fetch_and_save_metadata() {
    status_code=$(curl -L -s -w "%{http_code}" -o "$METADATA_FILE" ${HF_TOKEN:+-H "Authorization: Bearer $HF_TOKEN"} "$API_URL")
    RESPONSE=$(cat "$METADATA_FILE")
    if [ "$status_code" -eq 200 ]; then
        printf "%s\n" "$RESPONSE"
    else
        printf "%b[Error] Failed to fetch metadata from %s. HTTP status code: %s.%b\n%s\n" "$RED" "$API_URL" "$status_code" "$NC" "$RESPONSE" >&2
        rm -f "$METADATA_FILE"
        exit 1
    fi
}

# Exit early if the repo is gated/private but no credentials were supplied.
check_authentication() {
    local gated
    if command -v jq &>/dev/null; then
        gated=$(printf '%s' "$1" | jq -r '.gated // false')
    else
        printf '%s' "$1" | grep -q '"gated":[^f]' && gated=true || gated=false
    fi
    if [[ "$gated" != "false" && ( -z "$HF_TOKEN" || -z "$HF_USERNAME" ) ]]; then
        printf "%bThe repository requires authentication, but --hf_username and --hf_token is not passed. Please get token from https://huggingface.co/settings/tokens.\nExiting.\n%b" "$RED" "$NC"
        exit 1
    fi
}

printf "%b%s%b (%s)\n" "$BOLD" "$REPO_ID" "$NC" "$REVISION"
if [[ ! -f "$METADATA_FILE" ]]; then
    printf "%bFetching metadata...%b\n" "$DIM" "$NC"
    RESPONSE=$(fetch_and_save_metadata) || exit 1
else
    RESPONSE=$(cat "$METADATA_FILE")
fi
check_authentication "$RESPONSE"

# Total bytes of the revision (cheap, branch-specific); empty if the endpoint lacks this API.
fetch_treesize() {
    local json
    json=$(curl -sSL ${HF_TOKEN:+-H "Authorization: Bearer $HF_TOKEN"} "$HF_ENDPOINT/api/$REPO_API_PATH/treesize/$REVISION") || return
    if command -v jq &>/dev/null; then
        printf '%s' "$json" | jq -r '.size // empty' 2>/dev/null
    else
        printf '%s' "$json" | grep -o '"size":[0-9]*' | head -1 | grep -o '[0-9]*'
    fi
}
# Reuse the cached list only if the command is unchanged AND the manifest is intact (its line
# count equals repo_info's total). An interrupted/failed walk leaves no repo_info, so it re-lists.
should_regenerate_filelist() {
    local cmd="$LOCAL_DIR/.hfd/last_download_command" mf="$LOCAL_DIR/.hfd/manifest" info="$LOCAL_DIR/.hfd/repo_info"
    [[ -f "$mf" && -f "$info" && "$(generate_command_string)" == "$(cat "$cmd" 2>/dev/null)" \
       && "$(wc -l < "$mf")" == "$(cut -d' ' -f1 "$info" 2>/dev/null)" ]] && return 1
    return 0
}

fileslist_file=".hfd/${TOOL}_urls.txt"

# Convert a list of wildcard patterns into a single alternation regex.
patterns_to_regex() {
    (($#)) || return 0
    printf '%s\n' "$@" | sed 's/\./\\./g; s/\*/.*/g' | paste -sd '|' -
}

# Emit "size<TAB>path" for every file in a tree page (jq, or a grep/awk fallback).
emit_page_files() {
    if command -v jq &>/dev/null; then
        jq -r '.[] | select(.type=="file") | "\(.size)\t\(.path)"' "$1"
    else
        # Strip the nested lfs{} object (it has its own "size") so type/size/path align per entry.
        sed 's/"lfs":{[^}]*}//g' "$1" \
            | grep -oE '"type":"[^"]*"|"size":[0-9]+|"path":"[^"]*"' \
            | sed 's/"type":"//; s/"size"://; s/"path":"//; s/"$//' \
            | awk 'NR%3==1{t=$0} NR%3==2{s=$0} NR%3==0{if(t=="file")print s"\t"$0}'
    fi
}

# Emit "size<TAB>path" for every file from the cached metadata siblings[] (needs jq).
emit_siblings() {
    jq -r '(.siblings // [])[] | "\(.size // 0)\t\(.rfilename)"' "$METADATA_FILE"
}

# True when siblings[] is the complete list: its sizes sum to treesize (so it wasn't truncated, as
# it is for very large repos). Lets us skip the slow tree walk for typical repos via one API call.
siblings_complete() {
    command -v jq &>/dev/null && [[ "$REPO_SIZE" =~ ^[0-9]+$ ]] || return 1
    local n s
    read -r n s < <(jq -r '(.siblings // []) as $s | "\($s|length) \([$s[].size // 0]|add // 0)"' "$METADATA_FILE" 2>/dev/null)
    (( ${n:-0} > 0 )) && [[ "${s:-0}" == "$REPO_SIZE" ]]
}

# Keep only "size<TAB>path" stdin lines matching include/exclude (the *_REGEX globals).
filter_size_path() {
    local size path
    while IFS=$'\t' read -r size path; do
        [[ -z "$path" ]] && continue
        [[ -n "$INCLUDE_REGEX" && ! "$path" =~ $INCLUDE_REGEX ]] && continue
        [[ -n "$EXCLUDE_REGEX" && "$path" =~ $EXCLUDE_REGEX ]] && continue
        printf '%s\t%s\n' "${size:-0}" "$path"
    done
}

# Resumably walk the recursive tree into .hfd/manifest.partial (filtered size<TAB>path). After each
# page, checkpoint .hfd/list_state (fingerprint / entries scanned / next cursor) so an interrupted or
# failed walk continues from there instead of restarting. Progress (entries scanned) goes to stderr.
walk_tree() {
    local state="$LOCAL_DIR/.hfd/list_state" partial="$LOCAL_DIR/.hfd/manifest.partial"
    local page="$LOCAL_DIR/.hfd/tree_page.json" fp url scanned=0 headers n saved_fp=""
    fp=$(generate_command_string)
    [[ -f "$state" && -f "$partial" ]] && { read -r saved_fp; read -r scanned; read -r url; } < "$state"
    if [[ "$saved_fp" != "$fp" ]]; then   # no checkpoint, or it belongs to a different command
        url="$HF_ENDPOINT/api/$REPO_API_PATH/tree/$REVISION?recursive=true&expand=false"
        scanned=0; : > "$partial"
    elif [[ -s "$partial" && -n "$(tail -c1 "$partial")" ]]; then
        sed -i '$d' "$partial"   # drop a torn final line from a kill mid-append (the page re-fetches)
    fi
    while [[ -n "$url" ]]; do
        headers=$(curl -sSL ${HF_TOKEN:+-H "Authorization: Bearer $HF_TOKEN"} -D - -o "$page" "$url") || return 1
        if command -v jq &>/dev/null; then n=$(jq 'length' "$page" 2>/dev/null); else n=$(grep -o '"type":"' "$page" | wc -l); fi
        emit_page_files "$page" | filter_size_path >> "$partial"
        scanned=$(( scanned + ${n:-0} ))
        printf '\r\033[K%bListing files... %d scanned%b' "$DIM" "$scanned" "$NC" >&2
        url=$(printf '%s' "$headers" | tr -d '\r' | grep -i '^link:' \
            | grep -o '<[^>]*>;[[:space:]]*rel="next"' | sed -n 's/^<\([^>]*\)>.*/\1/p')
        printf '%s\n%s\n%s\n' "$fp" "$scanned" "$url" > "$state"   # checkpoint after the page is in
    done
    return 0
}

# Dedup .hfd/manifest.partial by path (a resumed walk may re-list its boundary page) into the final
# manifest, and write "<count> <bytes>" totals to filelist_stats.
finalize_filelist() {
    local mf="$LOCAL_DIR/.hfd/manifest"
    : > "$mf"
    awk -F'\t' -v mf="$mf" '!seen[$2]++ { print >> mf; n++; s+=$1 } END { print (n+0)" "(s+0) }' \
        "$LOCAL_DIR/.hfd/manifest.partial" > "$LOCAL_DIR/.hfd/filelist_stats"
}

# Build tool input from the manifest; set NEED_COUNT (files still to fetch). One find pass diffs
# the tree, keeping files missing or the wrong size (.aria2 sidecar = in-progress, kept): cheap
# for huge repos, correct after a manual delete. Run from the local dir (manifest paths relative).
build_download_list() {
    local needed=.hfd/needed size path dir cur
    if [[ ! -s .hfd/manifest ]]; then NEED_COUNT=0; : > "$fileslist_file"; return; fi
    awk -F'\t' '
        FNR==NR { want[$2]=$1; ord[++n]=$2; next }
        { p=$2; sub(/^\.\//,"",p);
          if (p ~ /\.aria2$/) { sub(/\.aria2$/,"",p); part[p]=1 } else have[p]=$1 }
        END { for (i=1;i<=n;i++) { q=ord[i];
                if ((q in have) && have[q]==want[q] && !(q in part)) continue;
                print want[q] "\t" q } }
    ' .hfd/manifest <(find . -type f ! -path './.hfd/*' -printf '%s\t%p\n' 2>/dev/null) > "$needed"
    NEED_COUNT=$(wc -l < "$needed")
    # Per needed file: drop a wrong-size copy -c/--continue can't fix in place (aria2c with no
    # .aria2 control file, or a wget file larger than expected) for a fresh refetch; emit the record.
    while IFS=$'\t' read -r size path; do
        [[ -z "$path" ]] && continue
        if [[ "$TOOL" == "aria2c" ]]; then
            [[ -e "$path" && ! -e "$path.aria2" ]] && rm -f "$path"
            dir="${path%/*}"; [[ "$dir" == "$path" ]] && dir=""
            printf '%s/%s/resolve/%s/%s\n dir=%s\n out=%s\n' "$HF_ENDPOINT" "$DOWNLOAD_API_PATH" "$REVISION" "$path" "$dir" "${path##*/}"
            [[ -n "$HF_TOKEN" ]] && printf ' header=Authorization: Bearer %s\n' "$HF_TOKEN"
            printf '\n'
        else
            cur=$(stat -c%s "$path" 2>/dev/null || echo 0); (( cur > size )) && rm -f "$path"
            printf '%s/%s/resolve/%s/%s\n' "$HF_ENDPOINT" "$DOWNLOAD_API_PATH" "$REVISION" "$path"
        fi
    done < "$needed" > "$fileslist_file"
    rm -f "$needed"
}

if should_regenerate_filelist; then
    command -v jq &>/dev/null || printf "%b[Warning] jq not installed, using grep/awk for json parsing (slower). Consider installing jq.%b\n" "$YELLOW" "$NC"

    # Total size up front (before the long walk); label the call so its few-second wait isn't silent.
    printf "%bFetching repository info...%b" "$DIM" "$NC"
    REPO_SIZE=$(fetch_treesize); printf '\r\033[K'
    [[ "$REPO_SIZE" =~ ^[0-9]+$ ]] && printf "%bRepository size: %s%b\n" "$DIM" "$(human "$REPO_SIZE")" "$NC"
    INCLUDE_REGEX=$(patterns_to_regex "${INCLUDE_PATTERNS[@]}")
    EXCLUDE_REGEX=$(patterns_to_regex "${EXCLUDE_PATTERNS[@]}")
    printf "%bListing files...%b" "$DIM" "$NC"
    # Fast path: typical repos return their whole file list in the metadata; only big repos
    # (truncated siblings) need the paginated, resumable tree walk.
    if siblings_complete; then
        rm -f "$LOCAL_DIR/.hfd/list_state"
        emit_siblings | filter_size_path > "$LOCAL_DIR/.hfd/manifest.partial"
        gen_status=${PIPESTATUS[0]}
    else
        walk_tree; gen_status=$?
    fi
    # A failed walk keeps its checkpoint (list_state + manifest.partial), so a re-run resumes.
    (( gen_status == 0 )) || { printf "\n%b[Error] Failed to list repository files. Re-run to resume.%b\n" "$RED" "$NC" >&2; exit 1; }
    finalize_filelist
    read -r TOTAL_FILES SUM_SIZE < "$LOCAL_DIR/.hfd/filelist_stats"
    printf '\r\033[K%bListed %d files (%s)%b\n' "$DIM" "$TOTAL_FILES" "$(human "$SUM_SIZE")" "$NC"
    # Whole-revision total from treesize; the summed filtered size when include/exclude narrows it.
    TOTAL_SIZE=$REPO_SIZE
    [[ -n "$INCLUDE_REGEX$EXCLUDE_REGEX" || ! "$REPO_SIZE" =~ ^[0-9]+$ ]] && TOTAL_SIZE=$SUM_SIZE
    printf '%s %s\n' "$TOTAL_FILES" "$TOTAL_SIZE" > "$LOCAL_DIR/.hfd/repo_info"
    # Fingerprint written last: marks the list complete (an interrupted walk has none, so it re-lists).
    generate_command_string > "$LOCAL_DIR/.hfd/last_download_command"
    rm -f "$LOCAL_DIR/.hfd/manifest.partial" "$LOCAL_DIR/.hfd/list_state" "$LOCAL_DIR/.hfd/tree_page.json"
fi

cd "$LOCAL_DIR" || exit 1

# Render one in-place status line; speed = byte delta since last call (PREV_B/PREV_T).
# Fields are fixed-width so columns don't jitter as values grow a digit; ETA is last.
PREV_B=0; PREV_T=0
render_progress() {
    local now=$1 dfiles=$2 dt sp pct eta e
    dt=$(( SECONDS - PREV_T )); ((dt<1)) && dt=1
    sp=$(( (now - PREV_B) / dt )); ((sp<0)) && sp=0; PREV_B=$now; PREV_T=$SECONDS
    pct=0; ((TOTAL_SIZE>0)) && pct=$(( now * 100 / TOTAL_SIZE )); ((pct>100)) && pct=100
    # ETA only once speed is meaningful; clamp absurd early estimates to keep it tidy.
    eta="--:--"; ((sp>0 && TOTAL_SIZE>now)) && { e=$(( (TOTAL_SIZE-now)/sp )); ((e<360000)) && eta=$(printf '%02d:%02d' $((e/60)) $((e%60))); }
    printf '\r\033[K%b[%3d%%]%b %*d/%d files | %9s/%9s | %9s/s | ETA %s' \
        "$GREEN" "$pct" "$NC" "${#TOTAL_FILES}" "$dfiles" "$TOTAL_FILES" \
        "$(human "$now")" "$(human "$TOTAL_SIZE")" "$(human "$sp")" "$eta"
}

# Progress without scanning finished files: manifest totals + a stat of only in-flight files.
monitor_progress() {
    local interval=$1 base=$1 stop_hint="Stopping download, please wait..."; PREV_T=$SECONDS
    # The main INT trap is deferred until the foreground download returns, so acknowledge
    # Ctrl+C here (fires at once in the subshell) and note the force-stop that already works.
    [[ "$TOOL" == "aria2c" ]] && stop_hint="Stopping download, please wait... (press Ctrl+C again to force stop)"
    trap 'printf "\n%b%s%b\n" "$YELLOW" "$stop_hint" "$NC"; exit 0' INT
    if [[ "$TOOL" == "wget" ]]; then
        # wget downloads in manifest order, so a forward cursor needs to stat only the
        # current file; everything before it is done and counted by its known size.
        local -a MS MP; local s p
        while IFS=$'\t' read -r s p; do MS+=("$s"); MP+=("$p"); done < .hfd/manifest
        local idx=0 done_b=0 cur
        while :; do
            sleep "$interval"
            while (( idx < ${#MP[@]} )); do
                cur=$(stat -c%s "${MP[idx]}" 2>/dev/null) || break
                (( cur >= MS[idx] )) || break
                done_b=$(( done_b + MS[idx] )); idx=$((idx+1))
            done
            cur=0; (( idx < ${#MP[@]} )) && cur=$(stat -c%s "${MP[idx]}" 2>/dev/null || echo 0)
            render_progress $(( done_b + cur )) "$idx"
        done
    else
        # Parallel downloads finish out of order, so a cursor would miss files completing between
        # ticks. Each tick sums block usage (sparse-aware) of the manifest's files only — ignoring
        # stale files left from an earlier version of the repo — minus in-progress (.aria2) ones.
        local now files t0 walk
        while :; do
            sleep "$interval"
            t0=$SECONDS
            read -r now files < <(awk -F'\t' '
                FNR==NR { want[$2]=1; next }
                { p=$2; sub(/^\.\//,"",p)
                  if (p ~ /\.aria2$/) { sub(/\.aria2$/,"",p); if (p in want) a++; next }
                  if (p in want) { b+=$1; d++ } }
                END { print b*512, d-a }
            ' .hfd/manifest <(find . -type f ! -path './.hfd/*' -printf '%b\t%p\n' 2>/dev/null))
            render_progress "$now" "$files"
            # Self-throttle: if the walk took longer than the base interval, sleep that long
            # next time so scanning stays under ~half the wall-clock at any repo size.
            walk=$(( SECONDS - t0 )); interval=$base; (( walk > base )) && interval=$walk
        done
    fi
}

# Totals were persisted at file-list generation; reload for resumed runs.
[[ -f .hfd/repo_info ]] && read -r TOTAL_FILES TOTAL_SIZE < .hfd/repo_info
TOTAL_FILES=${TOTAL_FILES:-0}; TOTAL_SIZE=${TOTAL_SIZE:-0}
FILE_NOUN="files"; ((TOTAL_FILES==1)) && FILE_NOUN="file"

# Diff the local tree against the manifest to find what's missing, then short-circuit if done.
((TOTAL_FILES>50000)) && printf "%bChecking local files...%b\n" "$DIM" "$NC"
build_download_list
if (( NEED_COUNT == 0 )); then
    printf "%bUp to date. %s %s in %s%b\n" "$GREEN" "$TOTAL_FILES" "$FILE_NOUN" "$LOCAL_DIR" "$NC"
    exit 0
fi

# "Resuming" if any data is already on disk (completed files or a partial), else a fresh start.
verb="Downloading"; [[ -n "$(find . -type f ! -path './.hfd/*' -print -quit 2>/dev/null)" ]] && verb="Resuming"
printf "%s %s %s to %s  ·  Ctrl+C to stop, re-run to resume\n" "$verb" "$TOTAL_FILES" "$FILE_NOUN" "$LOCAL_DIR"

# Silence native per-file output (logged to .hfd/download.log) so the monitor owns one
# clean line. Refresh every 1s, backing off for huge repos where the walk gets expensive.
interval=1; ((TOTAL_FILES>50000)) && interval=5
monitor_progress "$interval" &
MON_PID=$!
trap 'kill "$MON_PID" 2>/dev/null' EXIT

if [[ "$TOOL" == "aria2c" ]]; then
    aria2c --quiet=true --log=.hfd/download.log --log-level=error --file-allocation=none \
        -x "$THREADS" -j "$CONCURRENT" -s "$THREADS" -k 1M -c -i "$fileslist_file" >/dev/null
    status=$?
else
    wget -x -nH --cut-dirs="$CUT_DIRS" ${HF_TOKEN:+--header="Authorization: Bearer $HF_TOKEN"} \
        --input-file="$fileslist_file" --continue -nv -o .hfd/download.log
    status=$?
fi

# Clear the live progress line in place; the final status line takes its spot (no blank line).
kill "$MON_PID" 2>/dev/null; wait "$MON_PID" 2>/dev/null; printf '\r\033[K'

if [[ $status -eq 0 ]]; then
    printf "%bDone. %s %s, %s in %s%b\n" "$GREEN" "$TOTAL_FILES" "$FILE_NOUN" "$(human "$TOTAL_SIZE")" "$LOCAL_DIR" "$NC"
else
    printf "%bDownload incomplete. Re-run to resume. Log: %s%b\n" "$RED" "$PWD/.hfd/download.log" "$NC"
    exit 1
fi
