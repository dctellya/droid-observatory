#!/usr/bin/env python3
"""Usage report tables for one milestone (owner item 18, rule 7): `make usage M=<milestone>`.

Reads the Droid CLI logs (droid-log-single.log*), the session sidecars (*.settings.json), the orchestrator
transcript, the mission progress_log.jsonl, the price table and the gate records, and writes the tables between
`<!-- usage:begin -->` and `<!-- usage:end -->` in reports/usage/<milestone>.md. Prose outside the markers is
never touched. Every number is computed here from those files; nothing is typed in. Python 3 standard library only.
"""

import argparse
import bisect
import collections
import glob
import json
import os
import re
import subprocess
import sys
import tomllib
from datetime import datetime, timedelta, timezone

BEGIN = "<!-- usage:begin -->"
END = "<!-- usage:end -->"

DEFAULT_MISSION_DIR = "/Users/dct/.factory/missions/e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c"
ROLES = ("orchestrator", "orchestrator subagent", "worker", "validator", "self-check", "other")
CLASSES = ("waits", "decisions and boundary reports", "narration", "unmatched")

# A log turn and its transcript record carry the same instant; m2 measured a largest difference of 0.003 s
# over 251 turns, so 3 s only absorbs clock rounding.
MATCH_TOLERANCE_S = 3.0
# Owner item 4 (M2 boundary): a wait that returns before this is reported as short.
SHORT_WAIT_S = 60.0
# Owner item 8: a feature finishes in one round under this many worker turns.
FEATURE_TURN_CAP = 120
# AGENTS.md "Mission setup": self-check missions run on the cheapest explicit model.
SELF_CHECK_MODEL = "claude-haiku-4-5-20251001"
# progress_log pauseReason values that mean the platform stopped the work (F-033, D-028).
THROTTLE_REASONS = {"unrecoverable_usage_402"}
VALIDATOR_FEATURE = re.compile(r"^(scrutiny|user-testing)-validator-")
# Tools whose use in an orchestrator turn is a decision or a boundary action (m2.md method, F-034).
DECISION_TOOLS = {"Edit", "Create", "Task", "TaskStop", "StartMissionRun", "DismissHandoffItems", "AskUser",
                  "ProposeMission", "ExitSpecMode", "ApplyPatch"}
SHELL_TOOLS = {"Execute", "Script"}
WRITE_RE = re.compile(r"(?<![<>=-])>{1,2}(?!=)|\btee\b|\bgit\s+(add|commit|merge|push|worktree|checkout|branch)\b|"
                      r"\b(mv|cp|rm|mkdir|touch)\b|sed\s+-i")
TS_LINE = re.compile(r"^\[(\d{4}-\d\d-\d\dT[\d:.]+Z)\]")


def parse_ts(ts):
    return datetime.fromisoformat(ts)


def sec(ts):
    return ts[:19]


def fmt_s(dt):
    return dt.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def log_sort_key(path):
    name = os.path.basename(path)
    m = re.match(r"droid-log-single\.log\.(\d{4}-\d\d-\d\d)(?:\.(\d+))?$", name)
    if not m:
        return ("9999-99-99", 0)
    return (m.group(1), int(m.group(2) or 0))


class Acc:
    __slots__ = ("turns", "i", "o", "cr", "cw", "rs", "first", "last", "files")

    def __init__(self):
        self.turns = self.i = self.o = self.cr = self.cw = self.rs = 0
        self.first = self.last = None
        self.files = set()

    def add(self, ts, d, fname):
        self.turns += 1
        self.i += d.get("inputTokens", 0)
        self.o += d.get("outputTokens", 0)
        self.cr += d.get("cacheReadInputTokens", 0)
        self.cw += d.get("cachedTokensWritten", 0)
        self.rs += d.get("reasoningTokens", 0)
        if self.first is None or ts < self.first:
            self.first = ts
        if self.last is None or ts > self.last:
            self.last = ts
        self.files.add(fname)


class Agg:
    def __init__(self):
        self.sessions = set()
        self.turns = self.i = self.o = self.cr = self.cw = self.rs = self.unpriced = 0
        self.usd = 0.0
        self.first = self.last = None
        self.files = set()
        self.models = {}

    def add(self, sid, model, a, usd):
        self.sessions.add(sid)
        self.turns += a.turns
        self.i += a.i
        self.o += a.o
        self.cr += a.cr
        self.cw += a.cw
        self.rs += a.rs
        if usd is None:
            self.unpriced += a.turns
        else:
            self.usd += usd
        if a.first is not None and (self.first is None or a.first < self.first):
            self.first = a.first
        if a.last is not None and (self.last is None or a.last > self.last):
            self.last = a.last
        self.files |= a.files
        if model not in self.models or a.first < self.models[model]:
            self.models[model] = a.first


def read_jsonl(path):
    out = []
    with open(path, errors="replace") as f:
        for n, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                out.append((n, json.loads(line)))
            except ValueError:
                continue
    return out


def load_prices(path):
    with open(path, "rb") as f:
        data = tomllib.load(f)
    return data.get("models", {})


def price(prices, model, a):
    p = prices.get(model)
    if p is None:
        return None
    return (a.i * p["input_per_m"] + a.o * p["output_per_m"] + a.cr * p["cache_read_per_m"]
            + a.cw * p["cache_write_per_m"]) / 1e6


def scan_logs(files, start, end):
    """One pass: per (session, model) sums for [start, end] and for everything up to end, plus router lines."""
    win, upto = {}, {}
    router = []
    info = []
    orch_turns = []
    unparsed = unjoined = 0
    for path in files:
        fname = os.path.basename(path)
        lines = 0
        first = last = None
        n_unparsed = 0
        with open(path, errors="replace") as f:
            for line in f:
                lines += 1
                m = TS_LINE.match(line)
                if not m:
                    continue
                ts = m.group(1)
                if first is None:
                    first = ts
                last = ts
                is_turn = "[Agent] Streaming result" in line
                if not is_turn and "[Model-Router] Model transition" not in line:
                    continue
                s = sec(ts)
                if s > sec(end):
                    continue
                idx = line.find(" | Context: ")
                try:
                    d = json.loads(line[idx + 12:])
                except ValueError:
                    n_unparsed += 1
                    continue
                if not is_turn:
                    if start is None or s >= sec(start):
                        router.append((ts, d, fname))
                    continue
                tags = d.get("tags") or {}
                sid = tags.get("sessionId")
                if not sid:
                    unjoined += 1
                    continue
                model = tags.get("modelId") or "unknown"
                key = (sid, model)
                upto.setdefault(key, Acc()).add(ts, d, fname)
                if start is None or s >= sec(start):
                    win.setdefault(key, Acc()).add(ts, d, fname)
                    if sid == scan_logs.orch_id:
                        orch_turns.append((ts, model, d))
        unparsed += n_unparsed
        info.append({"path": path, "lines": lines, "first": first, "last": last, "unparsed": n_unparsed})
    return win, upto, router, info, orch_turns, unparsed, unjoined


scan_logs.orch_id = None


def load_sidecars(sessions_dir):
    out = {}
    for path in sorted(glob.glob(os.path.join(sessions_dir, "*", "*.settings.json"))):
        sid = os.path.basename(path)[: -len(".settings.json")]
        try:
            with open(path) as f:
                out[sid] = (path, json.load(f))
        except (ValueError, OSError):
            continue
    return out


class Classifier:
    def __init__(self, sidecars, progress, mission_id):
        self.sidecars = sidecars
        self.mission_id = mission_id
        self.feature = {}
        for _, r in progress:
            if r.get("type") == "worker_started" and r.get("workerSessionId") not in self.feature:
                self.feature[r["workerSessionId"]] = r.get("featureId")
        self.memo = {}

    def of(self, sid, depth=0):
        if sid in self.memo:
            return self.memo[sid]
        res = self._of(sid, depth)
        self.memo[sid] = res
        return res

    def _worker_role(self, sid):
        feat = self.feature.get(sid)
        return "validator" if feat and VALIDATOR_FEATURE.match(feat) else "worker"

    def _of(self, sid, depth):
        base = {"role": "other", "kind": "root", "feature": self.feature.get(sid), "mission": None, "caller_role": None}
        tags = (self.sidecars.get(sid, (None, {}))[1].get("tags")) or []
        ms = next((t for t in tags if isinstance(t, dict) and t.get("name") == "mission-session"), None)
        sub = next((t for t in tags if isinstance(t, dict) and t.get("name") == "subagent"), None)
        if ms:
            meta = ms.get("metadata") or {}
            mid = meta.get("missionId") or self.mission_id
            if mid != self.mission_id:
                return dict(base, role="self-check", mission=mid)
            if meta.get("role") == "orchestrator":
                return dict(base, role="orchestrator")
            return dict(base, role=self._worker_role(sid))
        if sub and depth < 8:
            caller = (sub.get("metadata") or {}).get("callingSessionId")
            c = self.of(caller, depth + 1) if caller else {"role": "other", "mission": None}
            role = "orchestrator subagent" if c["role"] == "orchestrator" else c["role"]
            return dict(base, role=role, kind="subagent of " + (caller or "?")[:8], mission=c.get("mission"),
                        caller_role=c["role"], feature=c.get("feature") if c["role"] in ("worker", "validator") else None)
        if sid in self.feature:
            return dict(base, role=self._worker_role(sid))
        return base


def md_table(header, rows):
    out = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    if not rows:
        rows = [["none"] + ["-"] * (len(header) - 1)]
    for r in rows:
        out.append("| " + " | ".join(str(c).replace("|", "\\|") for c in r) + " |")
    return out


def usd(x):
    return "%.2f" % x


def ts_or_dash(x):
    return x if x else "-"


def agg_cells(a):
    return [a.turns, a.i, a.o, a.cr, a.cw, a.rs, usd(a.usd), a.unpriced]


def tool_names(content):
    """(names, text) of one assistant record; the calls inside a Script count as the turn's tools."""
    names, text, bg = set(), [], False
    for b in content:
        if b.get("type") == "text":
            text.append("text")
        if b.get("type") != "tool_use":
            continue
        n = b.get("name")
        inp = b.get("input") or {}
        if n == "Script":
            body = str(inp.get("script", ""))
            inner = set(re.findall(r"tools\.(\w+)", body))
            names |= inner or {"Script"}
            text.append(body)
            bg = bg or bool(inp.get("startInBackground"))
        else:
            names.add(n)
            if n == "Execute":
                text.append(str(inp.get("command", "")))
    return names, "\n".join(text), bg


def load_transcript(path):
    recs, results, waits = [], {}, []
    with open(path, errors="replace") as f:
        for line in f:
            if '"message"' not in line:
                continue
            try:
                r = json.loads(line)
            except ValueError:
                continue
            if r.get("type") != "message":
                continue
            m = r.get("message") or {}
            content = m.get("content")
            if not isinstance(content, list):
                continue
            ts = r.get("timestamp")
            if m.get("role") == "user":
                for b in content:
                    if b.get("type") == "tool_result":
                        results[b.get("tool_use_id")] = ts
                continue
            if m.get("role") != "assistant" or str(r.get("id", "")).startswith("repl-tool-call"):
                continue
            kinds = {b.get("type") for b in content}
            if not kinds & {"text", "tool_use"}:
                continue
            names, text, bg = tool_names(content)
            recs.append({"ts": ts, "t": parse_ts(ts).timestamp(), "names": names, "text": text, "bg": bg})
            for b in content:
                if b.get("type") == "tool_use" and b.get("name") == "WaitForScript":
                    waits.append((ts, b.get("id")))
    recs.sort(key=lambda r: r["t"])
    return recs, results, waits


def classify_turn(rec, prev):
    names = rec["names"]
    if not names:
        return CLASSES[1]
    if names <= {"WaitForScript"}:
        return CLASSES[0]
    if names <= {"StartMissionRun"} and prev is not None and prev["names"] <= {"WaitForScript", "StartMissionRun"}:
        return CLASSES[0]
    if rec["bg"] or names & DECISION_TOOLS or (names & SHELL_TOOLS and WRITE_RE.search(rec["text"])):
        return CLASSES[1]
    return CLASSES[2]


def orchestrator_split(orch_turns, recs, prices):
    times = [r["t"] for r in recs]
    rows = collections.OrderedDict()
    matched = 0
    prev = None
    for ts, model, d in sorted(orch_turns, key=lambda x: x[0]):
        t = parse_ts(ts).timestamp()
        pos = bisect.bisect_left(times, t)
        cands = [recs[j] for j in range(max(0, pos - 3), min(len(recs), pos + 3)) if abs(times[j] - t) <= MATCH_TOLERANCE_S]
        rec = min(cands, key=lambda r: (abs(r["t"] - t), not r["names"])) if cands else None
        if rec is None:
            cls = CLASSES[3]
        else:
            matched += 1
            cls = classify_turn(rec, prev)
            prev = rec
        a = Acc()
        a.add(ts, d, "")
        rows.setdefault((cls, model), []).append(a)
    result = {}
    for (cls, model), accs in rows.items():
        agg = Agg()
        for a in accs:
            agg.add("orch", model, a, price(prices, model, a))
        result[(cls, model)] = agg
    return result, matched


def throttled_rows(progress, label, start, end):
    rows, open_ = [], None

    def close(next_entry, resumed):
        ln, r = open_
        t0 = parse_ts(r["timestamp"])
        t1 = parse_ts(next_entry[1]["timestamp"])
        reason = r.get("pauseReason") or "-"
        resume = next_entry[1]["timestamp"] if resumed else "none (next: %s %s)" % (next_entry[1]["type"], next_entry[1]["timestamp"])
        rows.append({"pause": r["timestamp"], "resume": resume, "seconds": (t1 - t0).total_seconds(), "reason": reason,
                     "throttled": reason in THROTTLE_REASONS, "source": "%s:%d, :%d" % (label, ln, next_entry[0])})

    for ln, r in progress:
        t = r.get("type")
        if t == "mission_paused":
            if open_:
                close((ln, r), False)
            open_ = (ln, r)
        elif t == "mission_resumed" and open_:
            close((ln, r), True)
            open_ = None
        elif t == "mission_run_started" and open_:
            close((ln, r), False)
            open_ = None
    if open_:
        rows.append({"pause": open_[1]["timestamp"], "resume": "open (no later entry)", "seconds": None,
                     "reason": open_[1].get("pauseReason") or "-", "throttled": False, "source": "%s:%d" % (label, open_[0])})
    return [r for r in rows if sec(start) <= sec(r["pause"]) <= sec(end)]


def gate_rows(gate_dir, repo_root, start, end):
    rows = []
    for path in sorted(glob.glob(os.path.join(gate_dir, "*", "*.json"))):
        try:
            with open(path) as f:
                r = json.load(f)
        except (ValueError, OSError):
            continue
        st = r.get("started_at")
        if not st or not (sec(start) <= sec(st) <= sec(end)):
            continue
        r["_path"] = os.path.relpath(path, repo_root) if path.startswith(repo_root) else os.path.join(
            os.path.basename(gate_dir), os.path.relpath(path, gate_dir))
        rows.append(r)
    rows.sort(key=lambda r: (r["started_at"], r.get("name", "")))
    return rows


def minutes(a, b):
    return "%.2f" % ((parse_ts(b) - parse_ts(a)).total_seconds() / 60)


def cross_check(script_dir, files, start, end, per_session):
    sh = os.path.join(script_dir, "usage-from-log.sh")
    if not os.path.exists(sh):
        return ["cross-check: unavailable (scripts/usage-from-log.sh not found)"]
    ids = sorted(per_session)
    cmd = ["sh", sh] + list(files) + ["--embedded", "--from", start, "--to", end, "--session"] + ids
    proc = subprocess.run(cmd, capture_output=True, text=True)
    got = {}
    for line in proc.stdout.splitlines():
        if line.startswith("#") or not line.strip():
            continue
        p = line.split("\t")
        got[p[0]] = tuple(p[1:])
    diffs = []
    for sid in ids:
        want = per_session[sid]
        have = got.get(sid)
        if have != want:
            diffs.append("  %s: usage.py %s, usage-from-log.sh %s" % (sid, want, have))
    if proc.returncode == 0 and not diffs:
        return ["cross-check: identical (%d sessions, first/last ts, lines, input, output, cache read against scripts/usage-from-log.sh --embedded)" % len(ids)]
    return ["cross-check: DIFFERENT (exit %d)" % proc.returncode] + diffs[:50]


def build(args):
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    mission_dir = args.mission_dir
    mission_id = args.mission_id or os.path.basename(mission_dir.rstrip("/"))
    now = parse_ts(args.now) if args.now else datetime.now(timezone.utc)
    now = now.replace(microsecond=0)
    end = args.to or fmt_s(now - timedelta(seconds=1))

    pl_path = os.path.join(mission_dir, "progress_log.jsonl")
    progress = read_jsonl(pl_path) if os.path.exists(pl_path) else []
    pl_label = os.path.basename(mission_dir.rstrip("/")) + "/progress_log.jsonl"
    start = args.start
    if not start:
        firsts = [r["timestamp"] for _, r in progress
                  if r.get("type") == "worker_started" and str(r.get("featureId", "")).startswith(args.milestone + "-")]
        if not firsts:
            sys.exit("usage.py: no worker_started for %s-* in progress_log.jsonl; pass --from" % args.milestone)
        start = fmt_s(parse_ts(min(firsts)))

    files = sorted(glob.glob(os.path.join(args.logs_dir, "droid-log-single.log*")), key=log_sort_key)
    if not files:
        sys.exit("usage.py: no droid-log-single.log* in %s" % args.logs_dir)

    prices = load_prices(args.prices)
    sidecars = load_sidecars(args.sessions_dir)
    clf = Classifier(sidecars, progress, mission_id)
    scan_logs.orch_id = mission_id
    win, upto, router, loginfo, orch_turns, unparsed, unjoined = scan_logs(files, start, end)

    ms_path = os.path.join(mission_dir, "model-settings.json")
    ms = {}
    if os.path.exists(ms_path):
        with open(ms_path) as f:
            ms = json.load(f)
    by_default = lambda role: {m for m, p in prices.items() if p.get("role_default") == role}  # noqa: E731
    declared = {
        "orchestrator": by_default("orchestrator"),
        "orchestrator subagent": by_default("orchestrator"),
        "worker": {ms["workerModel"]} if ms.get("workerModel") else by_default("worker"),
        "validator": {ms["validationWorkerModel"]} if ms.get("validationWorkerModel") else by_default("validator"),
        "self-check": {SELF_CHECK_MODEL},
    }

    def aggregate(accs, keyfn):
        out = {}
        for (sid, model), a in accs.items():
            out.setdefault(keyfn(sid, model), Agg()).add(sid, model, a, price(prices, model, a))
        return out

    credit = lambda sid: ((sidecars.get(sid, (None, {}))[1].get("tokenUsage") or {}).get("factoryCredits") or 0)  # noqa: E731
    role_of = lambda sid: clf.of(sid)["role"]  # noqa: E731

    lines = []
    emit = lines.append
    emit("generated: %s" % fmt_s(now))
    emit("")
    emit("window: %s to %s (`--to` is the generation time minus 1 s unless given; log turns compare at second resolution, as `scripts/usage-from-log.sh` does)" % (start, end))
    emit("")
    emit("Directories: logs `%s`, sessions `%s`, mission `%s`, prices `%s`." % (
        args.logs_dir, args.sessions_dir, mission_dir, os.path.relpath(args.prices, repo_root) if args.prices.startswith(repo_root) else args.prices))
    emit("")

    # Sources
    emit("### Sources")
    emit("")
    src_rows = []
    for i in loginfo:
        src_rows.append(["`logs/%s`" % os.path.basename(i["path"]), i["lines"], ts_or_dash(i["first"]), ts_or_dash(i["last"]),
                         "unparsed %d" % i["unparsed"]])
    if progress:
        tss = [r["timestamp"] for _, r in progress if r.get("timestamp")]
        src_rows.append(["`%s`" % pl_label, len(progress), min(tss), max(tss), "records"])
    tpath = next(iter(glob.glob(os.path.join(args.sessions_dir, "*", mission_id + ".jsonl"))), None)
    recs = results = waits = None
    if tpath:
        recs, results, waits = load_transcript(tpath)
        src_rows.append(["`sessions/%s/%s.jsonl`" % (os.path.basename(os.path.dirname(tpath)), mission_id), len(recs),
                         ts_or_dash(recs[0]["ts"] if recs else None), ts_or_dash(recs[-1]["ts"] if recs else None),
                         "orchestrator transcript, assistant records"])
    src_rows.append(["`sessions/*/*.settings.json` (sidecars)", len(sidecars), "-", "-", "files"])
    src_rows.append(["`%s`" % os.path.basename(args.prices), len(prices), "-", "-", "models priced"])
    for r in md_table(["Source", "Lines", "First timestamp", "Last timestamp", "Note"], src_rows):
        emit(r)
    emit("")
    emit("Turns not joined to a session (no `tags.sessionId`): %d." % unjoined)
    emit("")
    if args.cross_check:
        per = {}
        for (sid, model), a in win.items():
            c = per.get(sid)
            if c is None:
                per[sid] = [a.first, a.last, a.turns, a.i, a.o, a.cr]
            else:
                c[0], c[1] = min(c[0], a.first), max(c[1], a.last)
                c[2] += a.turns
                c[3] += a.i
                c[4] += a.o
                c[5] += a.cr
        for r in cross_check(os.path.dirname(os.path.abspath(__file__)), files, start, end,
                             {k: tuple(str(x) for x in v) for k, v in per.items()}):
            emit(r)
        emit("")

    # Tokens per role
    by_role = aggregate(win, lambda sid, m: role_of(sid))
    emit("### Tokens per role")
    emit("")
    rows = []
    total = Agg()
    for role in ROLES:
        a = by_role.get(role) or Agg()
        credits = sum(credit(sid) for sid in a.sessions)
        rows.append([role, len(a.sessions)] + agg_cells(a)[:7] + [a.unpriced, credits, ts_or_dash(a.first), ts_or_dash(a.last),
                                                                  ", ".join(sorted(a.files)) or "-"])
        total.sessions |= a.sessions
        for k in ("turns", "i", "o", "cr", "cw", "rs", "unpriced", "usd"):
            setattr(total, k, getattr(total, k) + getattr(a, k))
        total.files |= a.files
        if a.first and (total.first is None or a.first < total.first):
            total.first = a.first
        if a.last and (total.last is None or a.last > total.last):
            total.last = a.last
    rows.append(["total", len(total.sessions)] + agg_cells(total)[:7] + [total.unpriced, sum(credit(s) for s in total.sessions),
                                                                         ts_or_dash(total.first), ts_or_dash(total.last), ", ".join(sorted(total.files)) or "-"])
    hdr = ["Role", "Sessions", "Turns", "Input", "Output", "Cache read", "Cache write", "Reasoning", "USD-eq", "Unpriced turns",
           "Credits (sidecar, cumulative)", "First ts", "Last ts", "Source"]
    for r in md_table(hdr, rows):
        emit(r)
    emit("")
    emit("USD-eq is priced per model from the price table (input, output, cache read, cache write per million tokens); turns on a model with no price row are counted in `Unpriced turns` and add nothing to USD-eq. Credits come from the session sidecars and are cumulative for the session, not windowed.")
    emit("")

    # Tokens per session
    by_sess = aggregate(win, lambda sid, m: sid)
    emit("### Tokens per session")
    emit("")
    srows = []
    for sid, a in sorted(by_sess.items(), key=lambda kv: (ROLES.index(role_of(kv[0])), kv[1].first, kv[0])):
        c = clf.of(sid)
        models = ", ".join(m for m, _ in sorted(a.models.items(), key=lambda kv: kv[1]))
        srows.append(["`%s`" % sid, c["role"], c["kind"], c["feature"] or "-", models] + agg_cells(a)[:7] + [a.unpriced, a.first, a.last,
                                                                                                            ", ".join(sorted(a.files))])
    for r in md_table(["Session", "Role", "Kind", "Feature", "Models", "Turns", "Input", "Output", "Cache read", "Cache write",
                       "Reasoning", "USD-eq", "Unpriced turns", "First ts", "Last ts", "Source"], srows):
        emit(r)
    emit("")

    # Sidecars
    emit("### Sidecar totals per session (cumulative)")
    emit("")
    crow = []
    for sid, a in sorted(by_sess.items(), key=lambda kv: (ROLES.index(role_of(kv[0])), kv[1].first, kv[0])):
        if sid not in sidecars:
            continue
        path, d = sidecars[sid]
        u = d.get("tokenUsage") or {}
        crow.append(["`%s`" % sid, role_of(sid), d.get("model", "-"), d.get("reasoningEffort", "-"), u.get("inputTokens", "-"),
                     u.get("outputTokens", "-"), u.get("cacheReadTokens", "-"), u.get("cacheCreationTokens", "-"),
                     u.get("thinkingTokens", "-"), u.get("factoryCredits", "-"), d.get("assistantActiveTimeMs", "-"),
                     "`%s/%s.settings.json`" % (os.path.basename(os.path.dirname(path)), sid)])
    for r in md_table(["Session", "Role", "Model", "Effort", "Input", "Output", "Cache read", "Cache write", "Thinking", "Credits",
                       "Active ms", "Source"], crow):
        emit(r)
    emit("")
    emit("The sidecar is the session's lifetime total, all models, not windowed; it is shown beside the log rows and never added to them (D-029).")
    emit("")

    # Orchestrator split
    emit("### Orchestrator split")
    emit("")
    if recs is not None:
        split, matched = orchestrator_split(orch_turns, recs, prices)
    else:
        split, matched = orchestrator_split(orch_turns, [], prices)
    srows = []
    for cls in CLASSES:
        for (c, model), a in sorted(split.items(), key=lambda kv: kv[1].first):
            if c != cls:
                continue
            srows.append([cls, model, a.turns, a.i, a.o, a.cr, a.cw, usd(a.usd), a.unpriced, a.first, a.last])
    for r in md_table(["Class", "Model", "Turns", "Input", "Output", "Cache read", "Cache write", "USD-eq", "Unpriced turns",
                       "First ts", "Last ts"], srows):
        emit(r)
    emit("")
    n_waits = sum(a.turns for (c, _), a in split.items() if c == "waits")
    short = in_win = 0
    for ts, tid in (waits or []):
        if not (sec(start) <= sec(ts) <= sec(end)):
            continue
        in_win += 1
        if tid in results and (parse_ts(results[tid]) - parse_ts(ts)).total_seconds() < SHORT_WAIT_S:
            short += 1
    emit("- re-issued waits: %d (orchestrator turns whose only tool call is `WaitForScript`, or a `StartMissionRun` repeated straight after a wait)" % n_waits)
    emit("- waits returning in under %d s: %d" % (SHORT_WAIT_S, short))
    emit("- WaitForScript calls in the window: %d" % in_win)
    emit("- orchestrator turns matched to the transcript: %d of %d (tolerance %.0f s)" % (matched, len(orch_turns), MATCH_TOLERANCE_S))
    emit("- sources: log turns of `%s` in the window; transcript %s" % (mission_id, "`%s`" % os.path.basename(tpath) if tpath else "not found"))
    emit("")

    # Subagents
    emit("### Subagents")
    emit("")
    sub = aggregate({k: a for k, a in win.items() if clf.of(k[0])["kind"] != "root"},
                    lambda sid, m: clf.of(sid)["caller_role"])
    rows = []
    for role in ROLES:
        a = sub.get(role)
        if a:
            rows.append([role, len(a.sessions), a.turns, a.i, a.o, a.cr, a.cw, usd(a.usd), a.unpriced, a.first, a.last])
    for r in md_table(["Caller role", "Sessions", "Turns", "Input", "Output", "Cache read", "Cache write", "USD-eq", "Unpriced turns",
                       "First ts", "Last ts"], rows):
        emit(r)
    emit("")

    # Self-check
    emit("### Self-check missions")
    emit("")
    sc = aggregate({k: a for k, a in win.items() if clf.of(k[0])["role"] == "self-check"}, lambda sid, m: clf.of(sid)["mission"])
    rows = []
    for mid, a in sorted(sc.items(), key=lambda kv: kv[1].first):
        rows.append(["`%s`" % mid, len(a.sessions), a.turns, a.i, a.o, a.cr, a.cw, usd(a.usd), a.unpriced,
                     ", ".join(m for m, _ in sorted(a.models.items(), key=lambda kv: kv[1])), a.first, a.last])
    for r in md_table(["Mission", "Sessions", "Turns", "Input", "Output", "Cache read", "Cache write", "USD-eq", "Unpriced turns",
                       "Models", "First ts", "Last ts"], rows):
        emit(r)
    emit("")

    # Model fallback
    emit("### Model fallback")
    emit("")
    fb = []
    for (sid, model), a in sorted(win.items(), key=lambda kv: (kv[1].first, kv[0])):
        role = role_of(sid)
        if role in declared and model not in declared[role]:
            fb.append(["`%s`" % sid, role, model, a.turns, a.i, a.o, a.cr, a.cw, a.first, a.last, ", ".join(sorted(a.files))])
    for r in md_table(["Session", "Role", "Model", "Turns", "Input", "Output", "Cache read", "Cache write", "First ts", "Last ts",
                       "Source"], fb):
        emit(r)
    emit("")
    emit("### Model-Router transitions")
    emit("")
    rt = []
    for ts, d, fname in router:
        sid = d.get("sessionId") or (d.get("tags") or {}).get("sessionId")
        if not sid or role_of(sid) == "other":
            continue
        rt.append([ts, "`%s`" % sid, role_of(sid), d.get("source", "-"), d.get("reason", "-"), d.get("previousModelId", "-"),
                   d.get("modelId", "-"), fname])
    for r in md_table(["Time", "Session", "Role", "Source", "Reason", "From", "To", "Source file"], rt):
        emit(r)
    emit("")
    n_fb = sum(r[3] for r in fb)
    emit("model fallback: none" if not fb and not rt else
         "model fallback: %d turns on non-declared models, %d router transitions" % (n_fb, len(rt)))
    emit("")
    emit("Declared models: " + "; ".join("%s `%s`" % (r, ", ".join(sorted(m))) for r, m in declared.items() if r != "orchestrator subagent") + ".")
    emit("")

    # Throttled
    emit("### Throttled")
    emit("")
    th = throttled_rows(progress, pl_label, start, end)
    rows = [[t["pause"], t["resume"], "-" if t["seconds"] is None else "%.3f" % t["seconds"], t["reason"],
             "yes" if t["throttled"] else "no", t["source"]] for t in th]
    for r in md_table(["Pause", "Resume", "Seconds", "Reason", "Throttled", "Source"], rows):
        emit(r)
    emit("")
    secs = sum(t["seconds"] for t in th if t["throttled"] and t["seconds"] is not None)
    emit("throttled: %.3f s = %.2f min" % (secs, secs / 60))
    emit("")

    # Gates
    emit("### Gates")
    emit("")
    gr = gate_rows(args.gate_dir, repo_root, start, end)
    rows = [[g.get("name"), str(g.get("tree", ""))[:12], g.get("exit_code"), "%.3f" % g.get("seconds", 0), g.get("started_at"),
             g.get("finished_at"), "`%s`" % g.get("session"), "`%s`" % g["_path"]] for g in gr]
    for r in md_table(["Gate", "Tree", "Exit", "Seconds", "Started", "Finished", "Session", "Source"], rows):
        emit(r)
    emit("")
    emit("gate runs in the window: %d" % len(gr))
    emit("gate seconds run: %.3f" % sum(g.get("seconds", 0) for g in gr))
    emit("served from cache: not recorded (`scripts/gate.sh` writes no record for a hit, so hits and the minutes they saved have no source)")
    emit("")

    # Validation overhead
    emit("### Validation overhead")
    emit("")
    pl_start, pl_end = {}, {}
    for _, r in progress:
        sid = r.get("workerSessionId")
        if r.get("type") == "worker_started":
            pl_start.setdefault(sid, r["timestamp"])
        elif r.get("type") in ("worker_completed", "worker_failed"):
            pl_end[sid] = r["timestamp"]
    rows, tot_log, tot_pl = [], 0.0, 0.0
    for sid, a in sorted(by_sess.items(), key=lambda kv: kv[1].first):
        c = clf.of(sid)
        if c["role"] != "validator" or c["kind"] != "root":
            continue
        m_log = (parse_ts(a.last) - parse_ts(a.first)).total_seconds() / 60
        tot_log += m_log
        st, en = pl_start.get(sid), pl_end.get(sid)
        m_pl = "-"
        if st and en:
            m_pl = minutes(st, en)
            tot_pl += float(m_pl)
        rows.append(["`%s`" % sid, c["feature"] or "-", a.first, a.last, "%.2f" % m_log, st or "-", en or "-", m_pl,
                     "`%s`; %s" % (", ".join(sorted(a.files)), pl_label)])
    for r in md_table(["Session", "Feature", "First turn", "Last turn", "Minutes (log)", "Started", "Completed",
                       "Minutes (progress_log)", "Source"], rows):
        emit(r)
    emit("")
    emit("validation overhead minutes (log): %.2f" % tot_log)
    emit("validation overhead minutes (progress_log): %.2f" % tot_pl)
    emit("")

    # Feature size
    emit("### Feature size")
    emit("")
    rounds = collections.Counter(r.get("featureId") for _, r in progress if r.get("type") == "worker_started")
    feats = {}
    for sid, a in by_sess.items():
        c = clf.of(sid)
        if c["role"] in ("worker", "validator") and c["kind"] == "root" and c["feature"]:
            f = feats.setdefault(c["feature"], [[], 0])
            f[0].append(sid)
            f[1] += a.turns
    rows = [[f, rounds[f], ", ".join("`%s`" % s for s in sorted(v[0])), v[1], "yes" if v[1] > FEATURE_TURN_CAP else "no"]
            for f, v in sorted(feats.items())]
    for r in md_table(["Feature", "Rounds", "Sessions", "Turns", "Over %d turns" % FEATURE_TURN_CAP], rows):
        emit(r)
    emit("")
    emit("Rounds are the `worker_started` entries for the feature in `%s`; turns are the model turns of its root sessions in the window." % pl_label)
    emit("")

    # Mission to date
    emit("### Mission to date by role")
    emit("")
    caps = dict(c.split("=", 1) for c in args.cap)
    td = aggregate(upto, lambda sid, m: role_of(sid))
    rows = []
    for role in ROLES:
        a = td.get(role) or Agg()
        cap = caps.get(role)
        over = "-" if cap is None else ("yes" if a.usd > float(cap) else "no")
        rows.append([role, len(a.sessions), a.turns, a.i, a.o, a.cr, a.cw, usd(a.usd), a.unpriced, cap if cap is not None else "-", over,
                     ts_or_dash(a.first), ts_or_dash(a.last)])
    for r in md_table(["Role", "Sessions", "Turns", "Input", "Output", "Cache read", "Cache write", "USD-eq", "Unpriced turns",
                       "Cap USD-eq", "Over cap", "First ts", "Last ts"], rows):
        emit(r)
    emit("")
    emit("Every log line up to the end of the window, no start bound; caps are those passed with `--cap role=USD`.")
    return "\n".join(lines)


def write_out(path, milestone, body):
    block = "%s\n## Generated usage tables\n\n%s\n%s" % (BEGIN, body, END)
    text = None
    if os.path.exists(path):
        with open(path) as f:
            text = f.read()
    if text is None:
        text = "# Usage: milestone %s\n\n%s\n" % (milestone, block)
    elif BEGIN in text and END in text and text.index(BEGIN) < text.index(END):
        text = text[: text.index(BEGIN)] + block + text[text.index(END) + len(END):]
    else:
        text = text.rstrip("\n") + "\n\n" + block + "\n"
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, "w") as f:
        f.write(text)


def main(argv=None):
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--milestone", "-m", required=True)
    ap.add_argument("--mission-dir", default=os.environ.get("OBSERVATORY_MISSION_DIR", DEFAULT_MISSION_DIR))
    ap.add_argument("--mission-id", help="orchestrator session id; default the mission directory's name")
    ap.add_argument("--logs-dir", default=os.path.expanduser("~/.factory/logs"))
    ap.add_argument("--sessions-dir", default=os.path.expanduser("~/.factory/sessions"))
    prices = sorted(glob.glob(os.path.join(repo_root, "config", "prices", "*.toml")))
    ap.add_argument("--prices", default=prices[-1] if prices else None)
    ap.add_argument("--gate-dir", default=os.path.join(repo_root, "reports", "gate"))
    ap.add_argument("--from", dest="start", help="window start (RFC 3339); default the milestone's first worker_started")
    ap.add_argument("--to", help="window end; default the generation time minus 1 s")
    ap.add_argument("--now", help="generation time (tests)")
    ap.add_argument("--out", help="default reports/usage/<milestone>.md")
    ap.add_argument("--cap", action="append", default=[], metavar="ROLE=USD", help="cap to compare mission-to-date with")
    ap.add_argument("--cross-check", action="store_true", help="compare the session rows with scripts/usage-from-log.sh")
    args = ap.parse_args(argv)
    if not args.prices:
        sys.exit("usage.py: no price table found; pass --prices")
    out = args.out or os.path.join(repo_root, "reports", "usage", args.milestone + ".md")
    body = build(args)
    write_out(out, args.milestone, body)
    print("usage: wrote %s" % out)


if __name__ == "__main__":
    main()
