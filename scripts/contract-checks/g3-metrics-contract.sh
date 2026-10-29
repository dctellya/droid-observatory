#!/bin/sh
# VAL-METRIC-001, -002, -004, -006, -007, -009: the metrics CSV carries the 22-column contract (metrics-header.txt, 8
# waste_ columns, price_table last), every field obeys its rule, and the JSON report has the same (scope, id) rows with
# equal values and a non-null overhead line. The output comes from `observatory-cli metrics <fixture>` over the M0 capture;
# CONTRACT_METRICS_CSV and CONTRACT_METRICS_JSON replace it (the self-test). The command lands with M5: until then pending.
. "$(dirname "$0")/lib.sh"
ids="VAL-METRIC-001 VAL-METRIC-002 VAL-METRIC-004 VAL-METRIC-006 VAL-METRIC-007 VAL-METRIC-009"
need_tool jq python3
csv=${CONTRACT_METRICS_CSV:-}
json=${CONTRACT_METRICS_JSON:-}
if [ -z "$csv" ]; then
  if ! grep -qs -E '"metrics"|Metrics' crates/observatory-cli/src/main.rs; then
    pending "observatory-cli has no metrics command yet"
    finish "$ids"
  fi
  fx=fixtures/streams/2026-10-22-observatory-v1-m0.jsonl
  tmpd=$(mktemp -d "${TMPDIR:-/tmp}/contract-metrics.XXXXXX")
  trap 'rm -rf "$tmpd"' EXIT
  cargo run -q -p observatory-cli -- metrics "$fx" --csv > "$tmpd/m.csv" 2> /dev/null || fail "metrics --csv exits non-zero"
  cargo run -q -p observatory-cli -- metrics "$fx" --json > "$tmpd/m.json" 2> /dev/null || fail "metrics --json exits non-zero"
  csv=$tmpd/m.csv
  json=$tmpd/m.json
fi
[ -z "$FAILS" ] || finish "$ids"
header=$(cat "$(dirname "$0")/metrics-header.txt")
python3 - "$csv" "$header" > "${TMPDIR:-/tmp}/contract-metrics-csv.$$" 2>&1 << 'PY'
import csv, re, sys
path, header = sys.argv[1], sys.argv[2].split(",")
rows = list(csv.reader(open(path, newline=""), dialect="excel"))
if not rows or rows[0] != header:
    print("header differs from metrics-header.txt: %s" % (",".join(rows[0]) if rows else "empty"))
    sys.exit(0)
if len(header) != 22:
    print("metrics-header.txt has %d columns, wanted 22" % len(header))
if sum(1 for c in header if c.startswith("waste_")) != 8:
    print("the header does not carry 8 waste_ columns")
if header[-1] != "price_table":
    print("price_table is not the last column")
idx = {c: i for i, c in enumerate(header)}
ints = [c for c in header if c.endswith("_s") or c.startswith("tokens_") or c in ("throttled_minutes", "rework_windows")]
for n, r in enumerate(rows[1:], start=2):
    if len(r) != len(header):
        print("row %d has %d fields, wanted %d" % (n, len(r), len(header)))
        continue
    if r[idx["scope"]] not in ("feature", "milestone"):
        print("row %d: scope '%s' is not feature or milestone" % (n, r[idx["scope"]]))
    for c in ints:
        if not re.fullmatch(r"[0-9]+", r[idx[c]]):
            print("row %d: %s '%s' is not an integer" % (n, c, r[idx[c]]))
    if not re.fullmatch(r"[0-9]+(\.[0-9]+)?", r[idx["cost_usd"]]):
        print("row %d: cost_usd '%s' is not a decimal" % (n, r[idx["cost_usd"]]))
    if r[idx["price_table"]] != "2026-10-22":
        print("row %d: price_table '%s' is not 2026-10-22" % (n, r[idx["price_table"]]))
PY
while IFS= read -r line; do fail "$line"; done < "${TMPDIR:-/tmp}/contract-metrics-csv.$$"
rm -f "${TMPDIR:-/tmp}/contract-metrics-csv.$$"
[ -z "$FAILS" ] || finish "$ids"
jq -e . "$json" > /dev/null 2>&1 || { fail "metrics --json does not parse"; finish "$ids"; }
[ "$(jq -r '.overhead' "$json")" != null ] || fail "the overhead line is null under the default config"
# One (scope, id) -> value table per output, compared field by field for the columns the JSON must repeat.
fields="cycle_s first_pass_active_s rework_s rework_windows throttled_minutes tokens_input tokens_output tokens_cache_read tokens_cache_write cost_usd price_table"
python3 - "$csv" "$json" "$fields" > "${TMPDIR:-/tmp}/contract-metrics-join.$$" 2>&1 << 'PY'
import csv, json, sys
rows = list(csv.DictReader(open(sys.argv[1], newline="")))
doc = json.load(open(sys.argv[2]))
fields = sys.argv[3].split()
found = {}
def walk(o):
    if isinstance(o, dict):
        if "scope" in o and "id" in o:
            found[(o["scope"], o["id"])] = o
        for v in o.values():
            walk(v)
    elif isinstance(o, list):
        for v in o:
            walk(v)
walk(doc)
if not found:
    print("PENDING the JSON report has no (scope, id) rows to join")
    sys.exit(0)
for r in rows:
    j = found.get((r["scope"], r["id"]))
    if j is None:
        print("%s %s: no JSON counterpart" % (r["scope"], r["id"]))
        continue
    for f in fields:
        if f not in j:
            print("%s %s: JSON lacks %s" % (r["scope"], r["id"], f))
        elif f in ("price_table",):
            if str(j[f]) != r[f]:
                print("%s %s: %s differs (csv %s, json %s)" % (r["scope"], r["id"], f, r[f], j[f]))
        elif abs(float(j[f]) - float(r[f])) > (0.005 if f == "cost_usd" else 0):
            print("%s %s: %s differs (csv %s, json %s)" % (r["scope"], r["id"], f, r[f], j[f]))
PY
while IFS= read -r line; do
  case "$line" in PENDING*) pending "${line#PENDING }" ;; *) fail "$line" ;; esac
done < "${TMPDIR:-/tmp}/contract-metrics-join.$$"
rm -f "${TMPDIR:-/tmp}/contract-metrics-join.$$"
finish "$ids"
