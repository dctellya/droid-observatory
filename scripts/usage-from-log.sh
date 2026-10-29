#!/bin/sh
# Sum per-turn token usage per session from the Droid CLI log (F-034, D-029).
#
#   scripts/usage-from-log.sh <log...> --session <id>... [--from <ts>] [--to <ts>] [--embedded] [--prices <toml>]
#
# <log...> are given oldest first (droid-log-single.log.<date> before droid-log-single.log).
# One tab-separated row per session id:
#   session  first_ts  last_ts  lines  input  output  cache_read  cache_write
# followed by '#' lines that state the coverage of the logs read (first and last line
# timestamps, line counts, turns that could not be joined).
#
# Join (v1 heuristic): a "[Agent] Streaming result" line belongs to the sessionId of the nearest
# preceding "[metrics_log_*]" line that has one, if both lines carry the same second. With
# --embedded the line's own "tags":{..."sessionId":...} is used instead (Droid 0.247.0 writes it);
# the '#' lines then report how often the two joins disagree.
# --from/--to keep only turns whose timestamp is inside [from, to] (RFC 3339, second resolution).
# --prices <toml> checks every model of a counted turn (the turn's "tags":{..."modelId":...}) against the [models."<id>"]
# rows of a price table (config/prices/<date>.toml): a model with no row prints its name on stderr and exits 2, before any row.
set -eu

usage() {
  echo "usage: $0 <log...> --session <id>... [--from <ts>] [--to <ts>] [--embedded] [--prices <toml>]" >&2
  exit 2
}

logs=""
sessions=""
from=""
to=""
embedded=0
prices=""
mode=logs
while [ $# -gt 0 ]; do
  case "$1" in
    --session) mode=sessions ;;
    --from) [ $# -ge 2 ] || usage; from="$2"; shift ;;
    --to) [ $# -ge 2 ] || usage; to="$2"; shift ;;
    --embedded) embedded=1 ;;
    --prices) [ $# -ge 2 ] || usage; prices="$2"; shift ;;
    -*) usage ;;
    *)
      if [ "$mode" = logs ]; then logs="$logs $1"; else sessions="$sessions $1"; fi
      ;;
  esac
  shift
done
[ -n "$logs" ] && [ -n "$sessions" ] || usage

for f in $logs; do
  [ -r "$f" ] || { echo "$0: cannot read $f" >&2; exit 1; }
done

priced=""
if [ -n "$prices" ]; then
  [ -r "$prices" ] || { echo "$0: cannot read $prices" >&2; exit 1; }
  priced=$(sed -n 's/^\[models\."\([^"]*\)"\].*/\1/p' "$prices" | tr '\n' ' ')
fi

# shellcheck disable=SC2086
awk -v sessions="$sessions" -v from="$from" -v to="$to" -v embedded="$embedded" -v priced="$priced" -v prices="$prices" -v prog="$0" '
function field(line, key,    pat) {
  pat = "\"" key "\":[0-9]+"
  if (match(line, pat)) return substr(line, RSTART + length(key) + 3, RLENGTH - length(key) - 3) + 0
  return 0
}
function sid_of(line,    s) {
  if (match(line, /"sessionId":"[^"]+"/)) {
    s = substr(line, RSTART + 13, RLENGTH - 14)
    return s
  }
  return ""
}
function model_of(line) {
  if (match(line, /"modelId":"[^"]+"/)) return substr(line, RSTART + 11, RLENGTH - 12)
  return ""
}
BEGIN {
  n = split(sessions, ids, " ")
  for (i = 1; i <= n; i++) want[ids[i]] = 1
  m = split(priced, pm, " ")
  for (i = 1; i <= m; i++) known[pm[i]] = 1
}
FNR == 1 { meta_sid = ""; meta_sec = "" }
{
  ts = substr($0, 2, 24)
  if (ts !~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]T/) next
  total_lines++
  if (log_first == "") log_first = ts
  log_last = ts
  sec = substr(ts, 1, 19)
}
/^\[[^]]*\] [A-Z]+: \[metrics_log_/ {
  s = sid_of($0)
  if (s != "") { meta_sid = s; meta_sec = sec }
  next
}
index($0, "[Agent] Streaming result") {
  turns++
  if (from != "" && sec < substr(from, 1, 19)) next
  if (to != "" && sec > substr(to, 1, 19)) next
  joined = (sec == meta_sec) ? meta_sid : ""
  own = sid_of($0)
  if (own != "" && joined != "" && own != joined) disagree++
  if (own == "" && embedded) { unjoined++; next }
  sid = embedded ? own : joined
  if (sid == "") { if (!embedded) unjoined++; next }
  if (!(sid in want)) next
  if (!(sid in first)) first[sid] = ts
  last[sid] = ts
  lines[sid]++
  inp[sid] += field($0, "inputTokens")
  out[sid] += field($0, "outputTokens")
  cr[sid] += field($0, "cacheReadInputTokens")
  cw[sid] += field($0, "cachedTokensWritten")
  mdl = model_of($0)
  if (mdl != "" && !(mdl in seen_model)) { seen_model[mdl] = 1; model_order[++n_models] = mdl }
}
END {
  if (prices != "") {
    for (i = 1; i <= n_models; i++)
      if (!(model_order[i] in known)) {
        printf "%s: no price for model %s in %s\n", prog, model_order[i], prices > "/dev/stderr"
        bad = 1
      }
    if (bad) exit 2
  }
  for (i = 1; i <= n; i++) {
    s = ids[i]
    if (s in lines)
      printf "%s\t%s\t%s\t%d\t%d\t%d\t%d\t%d\n", s, first[s], last[s], lines[s], inp[s], out[s], cr[s], cw[s]
    else
      printf "%s\t-\t-\t0\t0\t0\t0\t0\n", s
  }
  printf "# logs first_line=%s last_line=%s lines=%d streaming_result_lines=%d\n", log_first, log_last, total_lines, turns
  printf "# join=%s unjoined=%d embedded_vs_metrics_disagree=%d\n", embedded ? "embedded" : "metrics", unjoined + 0, disagree + 0
}
' $logs
