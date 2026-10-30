#!/usr/bin/env bash
# Shell-level smoke test of attach (VAL-BOARD-027..032): starts the built Tauri binary against a
# temp sessions directory and a temp OBSERVATORY_CONFIG, feeds it session files the way Droid
# does, and checks the app log and the store it wrote against the CLI as the oracle.
#
#   scripts/tests/attach-smoke.sh [latency|limit|toolarge]      (default: latency)
#
# Environment:
#   SMOKE_APP       the app binary (default $CARGO_TARGET_DIR/debug/observatory-app; build it with
#                   `cargo build -p observatory-app` first)
#   SMOKE_LINES     lines of the M0 root fixture appended in `latency` (default 200; 2820 is all)
#   SMOKE_GAP_MS    gap between appends (default 20)
#   SMOKE_KEEP=1    keep the work directory
# The app opens a window whose page cannot load without `trunk serve`; the shell does not need it.
set -euo pipefail

case_name="${1:-latency}"
root="$(git rev-parse --show-toplevel)"
target="${CARGO_TARGET_DIR:-$root/target}"
app="${SMOKE_APP:-$target/debug/observatory-app}"
lines="${SMOKE_LINES:-200}"
gap_ms="${SMOKE_GAP_MS:-20}"
fixture="$root/fixtures/streams/2026-10-22-observatory-v1-m0"
cli=("cargo" "run" "-q" "-p" "observatory-cli" "--")

[ -x "$app" ] || { echo "attach-smoke: $app not found; run cargo build -p observatory-app" >&2; exit 2; }
work="$(mktemp -d "${TMPDIR:-/tmp}/attach-smoke.XXXXXX")"
sessions="$work/sessions/-scratch"
mkdir -p "$sessions" "$work/logs" "$work/oracle"
session_id="$(python3 -c 'import json,sys; print(json.loads(open(sys.argv[1]).readline())["id"])' "$fixture.jsonl")"
pid=""
cleanup() {
    [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
    [ "${SMOKE_KEEP:-0}" = 1 ] || rm -rf "$work"
}
trap cleanup EXIT

max_mb=512
[ "$case_name" = toolarge ] && max_mb=1
cat >"$work/config.toml" <<EOF
[ingest]
factory_sessions_dir = "$work/sessions"
factory_logs_dir = "$work/logs"
max_session_file_mb = $max_mb

[store]
path = "$work/store.sqlite"
EOF

start_app() {
    OBSERVATORY_CONFIG="$work/config.toml" "$app" >"$work/app.log" 2>&1 &
    pid=$!
    for _ in $(seq 1 100); do
        grep -q ' ingest_started' "$work/app.log" 2>/dev/null && return 0
        sleep 0.1
    done
    echo "attach-smoke: the app did not start the ingest thread" >&2
    cat "$work/app.log" >&2
    exit 1
}

stop_app() {
    kill -TERM "$pid" 2>/dev/null || true
    for _ in $(seq 1 50); do kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
    kill -KILL "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    pid=""
}

wait_quiet() {
    # No new events_appended line for two seconds.
    local last=-1 now
    for _ in $(seq 1 60); do
        now="$(grep -c events_appended "$work/app.log" || true)"
        [ "$now" = "$last" ] && return 0
        last="$now"
        sleep 2
    done
}

store_rows() {
    sqlite3 "$1" "SELECT kind||'|'||lane||'|'||at||'|'||coalesce(byte_offset,'')||'|'||sub FROM events ORDER BY at, session_id, byte_offset, sub, kind"
}

case "$case_name" in
latency)
    cp "$fixture.settings.json" "$sessions/$session_id.settings.json"
    : >"$sessions/$session_id.jsonl"
    start_app
    python3 - "$fixture.jsonl" "$sessions/$session_id.jsonl" "$lines" "$gap_ms" "$work/append.tsv" <<'PY'
import sys, time
src, dst, n, gap, log = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
with open(src, "rb") as f, open(dst, "ab") as out, open(log, "w") as t:
    for i, line in enumerate(f):
        if i >= n:
            break
        out.write(line)
        out.flush()
        t.write(f"{i + 1}\t{time.time() * 1000:.3f}\n")
        time.sleep(gap / 1000)
PY
    wait_quiet
    stop_app
    head -n "$lines" "$fixture.jsonl" >"$work/oracle/$session_id.jsonl"
    cp "$fixture.settings.json" "$work/oracle/$session_id.settings.json"
    "${cli[@]}" replay --root-only --store "$work/oracle.sqlite" "$work/oracle/$session_id.jsonl" >"$work/oracle.txt"
    cli_count="$(grep -vc '^unknown events:' "$work/oracle.txt")"
    store_count="$(sqlite3 "$work/store.sqlite" 'SELECT count(*) FROM events')"
    sqlite3 "$work/store.sqlite" 'SELECT byte_offset FROM events WHERE byte_offset IS NOT NULL ORDER BY byte_offset, sub' >"$work/offsets.txt"
    expected_name="$(python3 -c 'import json,sys; t=json.load(open(sys.argv[1])).get("tags") or []; print((t[0].get("name") if t else None) or sys.argv[2])' "$fixture.settings.json" "$session_id")"
    python3 - "$work/app.log" "$work/append.tsv" "$work/offsets.txt" "$fixture.jsonl" "$expected_name" <<'PY'
import bisect, re, sys, datetime as dt
log, appends, offsets, source, name = sys.argv[1:6]
def ts(text):
    return dt.datetime.fromisoformat(text.replace("Z", "+00:00")).timestamp() * 1000
lines = open(log).read().splitlines()
disc = [l for l in lines if " session_discovered" in l]
batches = [(ts(l.split()[0]), int(re.search(r"count=(\d+)", l).group(1))) for l in lines if " events_appended" in l]
sent = [float(l.split("\t")[1]) for l in open(appends)]
starts, at = [], 0
for i, raw in enumerate(open(source, "rb")):
    if i >= len(sent):
        break
    starts.append(at)
    at += len(raw)
print("session_discovered lines:", len(disc))
for l in disc:
    print("  ", l)
    assert f"name={name}" in l, "a session is named by its first tag, else by its id"
print("batches:", len(batches), "events in batches:", sum(c for _, c in batches))
# Events of one session arrive in file order, so batch k holds the next count events by offset.
events = [int(x) for x in open(offsets).read().split()]
delays, index = [], 0
for flushed, count in batches:
    for offset in events[index:index + count]:
        line = bisect.bisect_right(starts, offset) - 1
        delays.append(flushed - sent[line])
    index += count
delays.sort()
assert delays, "no event reached a batch"
p = lambda q: delays[min(len(delays) - 1, int(q * len(delays)))]
print(f"append-to-flush per event (ms), {len(delays)} events: p50={p(.5):.1f} p95={p(.95):.1f} max={delays[-1]:.1f}")
assert delays[-1] < 1000, "an event waited a second or more"
PY
    echo "events in the store: $store_count; CLI replay --root-only of the same $lines lines: $cli_count"
    store_rows "$work/store.sqlite" >"$work/live.rows"
    store_rows "$work/oracle.sqlite" >"$work/replay.rows.all"
    # Nothing in a session file marks its end: the usage the sidecar holds is read when the
    # session ends (the app exits normally), so a signal-killed app lacks those events.
    grep -v '^usage.reported|' "$work/replay.rows.all" >"$work/replay.rows" || true
    echo "replay events only a finished session gives (usage.reported): $(grep -c '^usage.reported|' "$work/replay.rows.all" || true)"
    if diff "$work/live.rows" "$work/replay.rows" >"$work/rows.diff"; then
        echo "kind|lane|at|byte_offset|sub per event: identical to the CLI replay ($store_count rows)"
    else
        echo "attach-smoke: live and replay rows differ:" >&2
        head -20 "$work/rows.diff" >&2
        exit 1
    fi
    ;;
limit)
    # 65 small sessions with ascending mtimes; the oldest must be the one left unwatched.
    python3 - "$fixture.jsonl" "$sessions" <<'PY'
import json, os, sys, time
head = open(sys.argv[1], "rb").read().splitlines(keepends=True)[:30]
now = time.time()
for i in range(1, 66):
    sid = "00000000-0000-4000-8000-%012d" % i
    first = json.loads(head[0])
    first["id"] = sid
    path = os.path.join(sys.argv[2], sid + ".jsonl")
    with open(path, "wb") as out:
        out.write((json.dumps(first) + "\n").encode())
        out.writelines(head[1:])
    os.utime(path, (now - (70 - i) * 60, now - (70 - i) * 60))
PY
    start_app
    sleep 4
    stop_app
    grep ' attach_status' "$work/app.log" | tail -1
    grep -q 'watched=64 unwatched=1' "$work/app.log" || { echo "attach-smoke: expected watched=64 unwatched=1" >&2; exit 1; }
    announced="$(grep -c ' session_discovered' "$work/app.log" || true)"
    echo "session_discovered lines: $announced (the oldest, ...0001, must not be among them)"
    [ "$announced" = 64 ] || { echo "attach-smoke: expected 64 sessions tailed" >&2; exit 1; }
    ! grep ' session_discovered' "$work/app.log" | grep -q 'session_id=00000000-0000-4000-8000-000000000001 ' || { echo "attach-smoke: the oldest session was tailed" >&2; exit 1; }
    ;;
toolarge)
    python3 - "$fixture.jsonl" "$sessions/$session_id.jsonl" <<'PY'
import sys
lines = open(sys.argv[1], "rb").read().splitlines(keepends=True)
with open(sys.argv[2], "wb") as out:
    size = 0
    for l in lines:
        if size >= 2 * 1024 * 1024:
            break
        out.write(l)
        size += len(l)
PY
    cp "$fixture.settings.json" "$sessions/$session_id.settings.json"
    start_app
    sleep 6
    stop_app
    grep ' TooLarge' "$work/app.log"
    warns="$(grep -c ' TooLarge' "$work/app.log" || true)"
    [ "$warns" = 1 ] || { echo "attach-smoke: expected one TooLarge line, saw $warns" >&2; exit 1; }
    cp "$sessions/$session_id.jsonl" "$work/oracle/"
    cp "$fixture.settings.json" "$work/oracle/$session_id.settings.json"
    "${cli[@]}" replay --root-only --store "$work/oracle.sqlite" "$work/oracle/$session_id.jsonl" >"$work/oracle.txt"
    echo "file: $(wc -c <"$sessions/$session_id.jsonl") bytes; events in the store: $(sqlite3 "$work/store.sqlite" 'SELECT count(*) FROM events'); CLI replay: $(grep -vc '^unknown events:' "$work/oracle.txt") (one of them is usage.reported, which only a finished session gives)"
    ;;
*)
    echo "usage: attach-smoke.sh [latency|limit|toolarge]" >&2
    exit 2
    ;;
esac
echo "attach-smoke $case_name: ok"
