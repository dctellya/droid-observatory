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

import usage_classes
import usage_idle
import usage_rewrites
import usage_testing

BEGIN = "<!-- usage:begin -->"
END = "<!-- usage:end -->"

REPRODUCIBILITY = """Reproducibility: the tables are a function of the files read and the window. The log is flushed with a lag
(m3-usage-target measured it), so a byte-for-byte rerun needs a --to at least 5 minutes before the generation time.
Without --to the window ends at the generation time minus 1 s and later runs differ. Sidecar readings can move while
a session is live: they are lifetime totals read at generation time, and the section lists the mtime of every sidecar
it read so a change can be traced to the file."""

DEFAULT_MISSION_DIR = "/Users/dct/.factory/missions/e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c"
ROLES = ("orchestrator", "orchestrator subagent", "worker", "validator", "self-check", "other")
CLASSES = ("waits", "decisions and boundary reports", "narration", "unmatched")

# A log turn and its transcript record carry the same instant; m2 measured a largest difference of 0.003 s
# over 251 turns, so 3 s only absorbs clock rounding.
MATCH_TOLERANCE_S = 3.0
# The Droid log is not time-ordered: m2 and m3 logs hold lines up to 48 s behind an earlier one. Every line is filtered by its
# own timestamp; the scan only stops once a line is this far past --to, as the log is flushed with a lag well below it.
LOG_REORDER_MARGIN_S = 600
# Owner item 4 (M2 boundary): a wait that returns before this is reported as short.
SHORT_WAIT_S = 60.0
RATE_KEYS = ("input", "output", "cache_read", "cache_write")
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

    def merge(self, other):
        self.sessions |= other.sessions
        for k in ("turns", "i", "o", "cr", "cw", "rs", "unpriced", "usd"):
            setattr(self, k, getattr(self, k) + getattr(other, k))
        if other.first and (self.first is None or other.first < self.first):
            self.first = other.first
        if other.last and (self.last is None or other.last > self.last):
            self.last = other.last
        self.files |= other.files


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


def fail(message):
    print("usage-report.py: " + message, file=sys.stderr)
    sys.exit(2)


def load_prices(path):
    return load_price_table(path)[0]


def load_price_table(path):
    """(model rows, provenance line or None) of a dated price file."""
    with open(path, "rb") as f:
        data = tomllib.load(f)
    return data.get("models", {}), data.get("provenance")


def pick_price_table(prices_dir, named=None):
    """The file `named` (a date, a file name or a path such as the `price_table` of the config), else the newest dated file of the directory."""
    if named:
        for cand in ([named] if os.path.isabs(named) else []) + [os.path.join(prices_dir, named), os.path.join(prices_dir, named + ".toml"),
                                                                  os.path.join(prices_dir, os.path.basename(named))]:
            if os.path.isfile(cand):
                return cand
        sys.exit("usage-report.py: no price table %s in %s" % (named, prices_dir))
    dated = sorted(glob.glob(os.path.join(prices_dir, "*.toml")))
    return dated[-1] if dated else None


def rates(model, p):
    """(basis, per-million rates by token class): the plan price when the row carries one, else the list price."""
    plan = ["plan_%s_per_m" % k for k in RATE_KEYS]
    if any(k in p for k in plan):
        if not all(k in p for k in plan):
            fail("the price row of %s has some plan_*_per_m keys but not all four" % model)
        return "plan", {k: p["plan_%s_per_m" % k] for k in RATE_KEYS}
    missing = [k + "_per_m" for k in RATE_KEYS if k + "_per_m" not in p]
    if missing:
        fail("the price row of %s lacks %s" % (model, ", ".join(missing)))
    return "list", {k: p[k + "_per_m"] for k in RATE_KEYS}


def check_priced(prices, models, path):
    """A model that ran in the window is priced, and not at zero: its turns never show at zero cost."""
    for model, turns in sorted(models.items()):
        p = prices.get(model)
        if p is None:
            fail("no price for model %s (%d turns in the window) in %s; add a [models.\"%s\"] row (plan or list price)"
                 % (model, turns, path, model))
        _, r = rates(model, p)
        if r["input"] <= 0 or r["output"] <= 0:
            fail("the price of model %s has a zero input or output rate in %s; its turns would show at zero cost" % (model, path))


def price(prices, model, a):
    p = prices.get(model)
    if p is None:
        return None
    _, r = rates(model, p)
    return (a.i * r["input"] + a.o * r["output"] + a.cr * r["cache_read"] + a.cw * r["cache_write"]) / 1e6


def scan_logs(files, start, end):
    """One pass: per (session, model) sums for [start, end] and for everything up to end, plus router lines."""
    win, upto = {}, {}
    router = []
    info = []
    orch_turns = []
    facts = usage_idle.LogFacts()
    unparsed = unjoined = 0
    stop = (parse_ts(sec(end) + "+00:00") + timedelta(seconds=LOG_REORDER_MARGIN_S)).strftime("%Y-%m-%dT%H:%M:%S")
    for path in files:
        fname = os.path.basename(path)
        lines = 0
        first = last = None
        n_unparsed = 0
        past_end = False
        with open(path, errors="replace") as f:
            for line in f:
                lines += 1
                m = TS_LINE.match(line)
                if not m:
                    lines -= past_end
                    continue
                ts = m.group(1)
                past_end = sec(ts) > sec(end)
                if past_end:
                    lines -= 1
                    if sec(ts) > stop:
                        break
                    continue
                if first is None:
                    first = ts
                last = ts
                is_turn = "[Agent] Streaming result" in line
                if start is None or sec(ts) >= sec(start):
                    if is_turn:
                        facts.turns.append(ts)
                    else:
                        facts.note(ts, line)
                if not is_turn and "[Model-Router] Model transition" not in line:
                    continue
                s = sec(ts)
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
    return win, upto, router, info, orch_turns, unparsed, unjoined, facts


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


def usd_nonzero(x):
    return "<0.01" if 0 < x < 0.005 else usd(x)


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


def text_of_result(content):
    if isinstance(content, str):
        return content
    return " ".join(b.get("text", "") for b in content or () if isinstance(b, dict) and b.get("type") == "text")


def load_transcript(path):
    """(assistant records, tool_result times, WaitForScript calls, owner-facing calls: asks, answers, messages)."""
    recs, results, waits = [], {}, []
    owner = {"asks": {}, "answers": {}, "messages": {}}
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
                        owner["answers"][b.get("tool_use_id")] = (ts, text_of_result(b.get("content")))
                continue
            if m.get("role") == "assistant":
                for b in content:
                    inp = b.get("input") or {}
                    if b.get("type") == "tool_use" and b.get("name") == "AskUser":
                        owner["asks"][b.get("id")] = (ts, json.dumps(inp))
                    elif b.get("type") == "tool_use" and b.get("name") == "StartMissionRun" and inp.get("messageToWorker"):
                        owner["messages"][b.get("id")] = (ts, inp.get("resumeWorkerSessionId"), str(inp["messageToWorker"]))
            if m.get("role") != "assistant" or str(r.get("id", "")).startswith("repl-tool-call"):
                continue
            kinds = {b.get("type") for b in content}
            if not kinds & {"text", "tool_use"}:
                continue
            names, text, bg = tool_names(content)
            recs.append({"ts": ts, "t": parse_ts(ts).timestamp(), "names": names, "text": text, "bg": bg})
            for b in content:
                if b.get("type") == "tool_use" and b.get("name") == "WaitForScript":
                    inp = b.get("input") or {}
                    waits.append((ts, b.get("id"), inp.get("toolCallId"), inp.get("action")))
    recs.sort(key=lambda r: r["t"])
    return recs, results, waits, owner


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


def pause_rows(progress, label):
    """Every pause of progress_log.jsonl paired with the entry that ended it; `until` is that entry's time (None while open)."""
    rows, open_ = [], None

    def close(next_entry, resumed):
        ln, r = open_
        t0 = parse_ts(r["timestamp"])
        t1 = parse_ts(next_entry[1]["timestamp"])
        reason = r.get("pauseReason") or "-"
        resume = next_entry[1]["timestamp"] if resumed else "none (next: %s %s)" % (next_entry[1]["type"], next_entry[1]["timestamp"])
        rows.append({"pause": r["timestamp"], "resume": resume, "seconds": (t1 - t0).total_seconds(), "reason": reason,
                     "throttled": reason in THROTTLE_REASONS, "source": "%s:%d, :%d" % (label, ln, next_entry[0]),
                     "until": next_entry[1]["timestamp"]})

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
                     "reason": open_[1].get("pauseReason") or "-", "throttled": False, "source": "%s:%d" % (label, open_[0]),
                     "until": None})
    return rows


def throttled_rows(progress, label, start, end):
    return [r for r in pause_rows(progress, label) if sec(start) <= sec(r["pause"]) <= sec(end)]


def gate_rows(gate_dir, repo_root, start, end):
    rows = []
    for path in sorted(glob.glob(os.path.join(gate_dir, "*", "*.json"))):
        try:
            with open(path) as f:
                r = json.load(f)
        except (ValueError, OSError):
            continue
        st, fin = r.get("started_at"), r.get("finished_at")
        if not (st or fin):
            continue
        if not st:
            r["started_at"] = st = fin
        fin = fin or st
        if not ((sec(st) < sec(end) or sec(fin) <= sec(end)) and sec(fin) >= sec(start)):
            continue
        r["_path"] = os.path.relpath(path, repo_root) if path.startswith(repo_root) else os.path.join(
            os.path.basename(gate_dir), os.path.relpath(path, gate_dir))
        rows.append(r)
    rows.sort(key=lambda r: (r["started_at"], r.get("name", "")))
    return rows


def gate_label(gate_dir, repo_root):
    return os.path.relpath(gate_dir, repo_root) if gate_dir.startswith(repo_root) else os.path.basename(gate_dir.rstrip("/"))


def hit_rows(gate_dir, start, end):
    """Cache hits from hits.log (scripts/gate.sh): time, name, tree, stored seconds, session, session_source."""
    rows = []
    try:
        with open(os.path.join(gate_dir, "hits.log")) as f:
            lines = f.read().splitlines()
    except OSError:
        return rows
    for line in lines:
        cells = line.split("\t")
        if len(cells) != 6:
            continue
        try:
            when, seconds = cells[0], float(cells[3])
            inside = sec(start) <= sec(when) <= sec(end)
        except ValueError:
            continue
        if inside:
            rows.append({"time": when, "name": cells[1], "tree": cells[2], "seconds": seconds,
                         "session": cells[4], "session_source": cells[5]})
    return rows


def hit_command(gate_dir, hit):
    try:
        with open(os.path.join(gate_dir, hit["tree"], hit["name"] + ".json")) as f:
            return json.load(f).get("command")
    except (ValueError, OSError):
        return None


def gates_summary(runs, hits, gate_dir, label):
    """Rows of the Gates table: per gate and in total, for gates run and gates served from cache."""
    rows = []
    for kind, items, src_of in (
            ("gates run", [(g.get("name"), g.get("command"), g.get("seconds", 0), g["started_at"], g.get("finished_at") or g["started_at"])
                           for g in runs], lambda n: "%s/*/%s.json" % (label, n)),
            ("gates served from cache", [(h["name"], hit_command(gate_dir, h), h["seconds"], h["time"], h["time"]) for h in hits],
             lambda n: "%s/hits.log" % label)):
        by_name = collections.OrderedDict()
        for it in sorted(items, key=lambda it: (str(it[0]), it[3])):
            by_name.setdefault(it[0], []).append(it)
        groups = [(n, v, src_of(n)) for n, v in by_name.items()]
        groups.append(("all", [it for v in by_name.values() for it in v], "%s/*/*.json" % label if kind == "gates run" else src_of(None)))
        for name, v, src in groups:
            cmds = sorted({c for _, c, _, _, _ in v if c})
            secs = sum(x[2] for x in v)
            rows.append([kind, name, "-" if name == "all" or not cmds else "; ".join("`%s`" % c for c in cmds), len(v),
                         "%.3f" % secs, "%.2f" % (secs / 60),
                         min((x[3] for x in v), default="-"), max((x[4] for x in v), default="-"), "`%s`" % src])
    return rows


def mtime_utc(path):
    return datetime.fromtimestamp(os.path.getmtime(path), timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def minutes(a, b):
    return "%.2f" % ((parse_ts(b) - parse_ts(a)).total_seconds() / 60)


def cross_check(script_dir, files, start, end, per_session, prices_path):
    sh = os.path.join(script_dir, "usage-from-log.sh")
    if not os.path.exists(sh):
        return ["cross-check: unavailable (scripts/usage-from-log.sh not found)"]
    ids = sorted(per_session)
    cmd = ["sh", sh] + list(files) + ["--embedded", "--prices", prices_path, "--from", start, "--to", end, "--session"] + ids
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
            diffs.append("  %s: usage-report.py %s, usage-from-log.sh %s" % (sid, want, have))
    if proc.returncode == 0 and not diffs:
        return ["cross-check: identical (%d sessions, first/last ts, lines, input, output, cache read, cache write and the price of every model against scripts/usage-from-log.sh --embedded)" % len(ids)]
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
    ft_path = os.path.join(mission_dir, "features.json")
    ft_label = os.path.basename(mission_dir.rstrip("/")) + "/features.json"
    skills, budgets, features = {}, {}, []
    if os.path.exists(ft_path):
        with open(ft_path) as f:
            features = [x for x in json.load(f).get("features", []) if "id" in x]
        skills = {x["id"]: x.get("skillName") or "-" for x in features}
        budgets = {x["id"]: usage_classes.turn_budget(x) for x in features}
    start = args.start
    if not start:
        firsts = [r["timestamp"] for _, r in progress
                  if r.get("type") == "worker_started" and str(r.get("featureId", "")).startswith(args.milestone + "-")]
        if not firsts:
            sys.exit("usage-report.py: no worker_started for %s-* in progress_log.jsonl; pass --from" % args.milestone)
        start = fmt_s(parse_ts(min(firsts)))

    files = sorted(glob.glob(os.path.join(args.logs_dir, "droid-log-single.log*")), key=log_sort_key)
    if not files:
        sys.exit("usage-report.py: no droid-log-single.log* in %s" % args.logs_dir)

    prices, provenance = load_price_table(args.prices)
    sidecars = load_sidecars(args.sessions_dir)
    clf = Classifier(sidecars, progress, mission_id)
    scan_logs.orch_id = mission_id
    win, upto, router, loginfo, orch_turns, unparsed, unjoined, facts = scan_logs(files, start, end)
    ran = collections.Counter()
    for (_, model), a in win.items():
        ran[model] += a.turns
    check_priced(prices, ran, os.path.relpath(args.prices, repo_root) if args.prices.startswith(repo_root) else args.prices)

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
                         "lines up to the window end; unparsed %d" % i["unparsed"]])
    if progress:
        tss = [r["timestamp"] for _, r in progress if r.get("timestamp") and sec(r["timestamp"]) <= sec(end)]
        src_rows.append(["`%s`" % pl_label, len(tss), min(tss) if tss else "-", max(tss) if tss else "-", "records up to the window end"])
    tpath = next(iter(glob.glob(os.path.join(args.sessions_dir, "*", mission_id + ".jsonl"))), None)
    recs = results = waits = None
    owner_calls = {}
    if tpath:
        recs, results, waits, owner_calls = load_transcript(tpath)
        bounded = [r["ts"] for r in recs if sec(r["ts"]) <= sec(end)]
        src_rows.append(["`sessions/%s/%s.jsonl`" % (os.path.basename(os.path.dirname(tpath)), mission_id), len(bounded),
                         ts_or_dash(bounded[0] if bounded else None), ts_or_dash(bounded[-1] if bounded else None),
                         "orchestrator transcript, assistant records up to the window end"])
    if skills:
        src_rows.append(["`%s`" % ft_label, len(skills), "-", "-", "feature id to skill (the Skill column of Feature size)"])
    n_sidecars = len({sid for sid, _ in upto if sid in sidecars})
    src_rows.append(["`sessions/*/*.settings.json` (sidecars)", n_sidecars, "-", "-", "sidecars of the sessions with a log turn up to the window end"])
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
                per[sid] = [a.first, a.last, a.turns, a.i, a.o, a.cr, a.cw]
            else:
                c[0], c[1] = min(c[0], a.first), max(c[1], a.last)
                c[2] += a.turns
                c[3] += a.i
                c[4] += a.o
                c[5] += a.cr
                c[6] += a.cw
        for r in cross_check(os.path.dirname(os.path.abspath(__file__)), files, start, end,
                             {k: tuple(str(x) for x in v) for k, v in per.items()}, args.prices):
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
        rows.append([role, len(a.sessions)] + agg_cells(a)[:7] + [a.unpriced, ts_or_dash(a.first), ts_or_dash(a.last),
                                                                  ", ".join(sorted(a.files)) or "-"])
        total.sessions |= a.sessions
        for k in ("turns", "i", "o", "cr", "cw", "rs", "unpriced", "usd"):
            setattr(total, k, getattr(total, k) + getattr(a, k))
        total.files |= a.files
        if a.first and (total.first is None or a.first < total.first):
            total.first = a.first
        if a.last and (total.last is None or a.last > total.last):
            total.last = a.last
    window_total = total
    rows.append(["total", len(total.sessions)] + agg_cells(total)[:7] + [total.unpriced,
                                                                         ts_or_dash(total.first), ts_or_dash(total.last), ", ".join(sorted(total.files)) or "-"])
    hdr = ["Role", "Sessions", "Turns", "Input", "Output", "Cache read", "Cache write", "Reasoning", "USD-eq", "Unpriced turns",
           "First ts", "Last ts", "Source"]
    for r in md_table(hdr, rows):
        emit(r)
    emit("")
    emit("USD-eq is priced per model from the price table (input, output, cache read, cache write per million tokens); a model that ran in the window with no price row, or a zero input or output price, stops the run (exit 2, naming the model), so `Unpriced turns` is 0 here; it only counts turns before the window (Mission to date by role). A price row with `plan_*_per_m` keys is the plan price and wins over the list price (see Models that ran).")
    emit("")

    # Cost by token class
    emit("### Cost by token class")
    emit("")
    scopes = collections.OrderedDict((role, usage_classes.new_totals()) for role in ROLES)
    milestone_totals = usage_classes.new_totals()
    for (sid, model), a in win.items():
        _, r = rates(model, prices[model])
        usage_classes.add_totals(scopes[role_of(sid)], a, r)
        usage_classes.add_totals(milestone_totals, a, r)
    crows = []
    for scope, totals in list(scopes.items()) + [("milestone", milestone_totals)]:
        if any(t for t, _ in totals.values()):
            crows += usage_classes.class_rows(scope, totals)
    for r in md_table(["Scope", "Class", "Tokens", "Share of tokens %", "USD-eq", "Share of cost %"], crows):
        emit(r)
    emit("")
    emit("Cost by token class: each role's and the milestone's tokens and USD-eq split by class, priced per model as in Tokens per role; "
         "share of tokens and share of cost are the class's part of that scope's total (cost follows the class, not the token count). "
         "Reasoning tokens are not a class: they are billed inside output.")
    emit("")

    # Models that ran
    emit("### Models that ran")
    emit("")
    mrows = []
    per_model = collections.OrderedDict()
    for (sid, model), a in sorted(win.items(), key=lambda kv: (kv[1].first, kv[0])):
        m = per_model.setdefault(model, [Acc(), 0.0, []])
        for k in ("turns", "i", "o", "cr", "cw", "rs"):
            setattr(m[0], k, getattr(m[0], k) + getattr(a, k))
        m[1] += price(prices, model, a)
        if role_of(sid) not in m[2]:
            m[2].append(role_of(sid))
    for model, (a, cost, roles) in per_model.items():
        basis, r = rates(model, prices[model])
        mrows.append([model, basis, a.turns, a.i, a.cw, a.cr, a.o, r["input"], r["cache_write"], r["cache_read"], r["output"],
                      usd_nonzero(cost), ", ".join(sorted(roles, key=ROLES.index)), prices[model].get("note") or "-"])
    for r in md_table(["Model", "Price basis", "Turns", "Input", "Cache write", "Cache read", "Output", "Input per M",
                       "Cache write per M", "Cache read per M", "Output per M", "USD-eq", "Roles", "Note"], mrows):
        emit(r)
    emit("")
    if provenance:
        emit("Price provenance: %s" % provenance)
        emit("")
    emit("Every model with a log turn in the window is listed, declared or routed. Price basis is `plan` when the model's row in `%s` "
         "carries the four `plan_*_per_m` keys (the plan's price) and `list` otherwise (the list price); per-million rates are USD."
         % os.path.basename(args.prices))
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
    short = in_win = reissued = kills = 0
    seen = set()
    for ts, tid, call_id, action in sorted(waits or [], key=lambda w: parse_ts(w[0])):
        inside = sec(start) <= sec(ts) <= sec(end)
        in_win += inside
        if action == "kill":
            kills += inside
            continue
        repeat = call_id in seen
        seen.add(call_id)
        if not inside:
            continue
        reissued += repeat
        if tid in results and (parse_ts(results[tid]) - parse_ts(ts)).total_seconds() < SHORT_WAIT_S:
            short += 1
    emit("- re-issued waits: %d (WaitForScript calls, any `action` but `kill`, on the same `toolCallId` as an earlier wait; "
         "the first wait on each id and every kill are not counted)" % reissued)
    emit("- worker pauses (kills): %d (WaitForScript calls whose `action` is `kill`: a worker or script stopped on purpose, "
         "not a wait)" % kills)
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
            rows.append([role, len(a.sessions), a.turns, a.i, a.o, a.cr, a.cw, usd(a.usd), a.unpriced, a.first, a.last,
                         ", ".join(sorted(a.files)) or "-"])
    for r in md_table(["Caller role", "Sessions", "Turns", "Input", "Output", "Cache read", "Cache write", "USD-eq", "Unpriced turns",
                       "First ts", "Last ts", "Source"], rows):
        emit(r)
    emit("")

    # Subagent classes
    emit("### Subagent classes")
    emit("")
    declarations = usage_classes.read_declarations(mission_dir, args.milestone)
    sub_models = collections.defaultdict(set)
    for (sid, model), a in win.items():
        if clf.of(sid)["kind"] != "root":
            sub_models[sid].add(model)
    srows, undeclared, subagent_sessions = [], 0, 0
    for cls, caller in (("orchestrator subagents", "orchestrator"), ("validator subagents", "validator"), ("worker subagents", "worker")):
        agg = Agg()
        for (sid, model), a in win.items():
            if clf.of(sid)["kind"] != "root" and clf.of(sid)["caller_role"] == caller:
                agg.add(sid, model, a, price(prices, model, a))
        models, purposes, sources, none, off = set(), set(), set(), 0, 0
        for sid in sorted(agg.sessions):
            d = usage_classes.declaration_of(declarations, sid)
            if d is None:
                none += 1
                continue
            models.add(d[0])
            purposes.add(d[1])
            sources.add(d[2])
            off += bool(sub_models[sid] - {d[0]})
        undeclared += none
        subagent_sessions += len(agg.sessions)
        srows.append([cls, len(agg.sessions), agg.turns, usd(agg.usd), ", ".join(sorted(models)) or "-", "; ".join(sorted(purposes)) or "-",
                      none, off, ", ".join("`%s`" % x for x in sorted(sources)) or "-"])
    for r in md_table(["Class", "Sessions", "Turns", "USD-eq", "Declared model", "Purpose", "No declared model", "Off declared model",
                       "Source"], srows):
        emit(r)
    emit("")
    emit("- no declared model: %d of %d subagent sessions" % (undeclared, subagent_sessions))
    emit("")
    emit("A subagent's class is the role of the session that started it. Declared model and purpose are read from `handoffs/*.json` and "
         "`validation/%s/**/synthesis*.json` in the mission directory (a `subagents` list or `subagentSessions` object, or a line "
         "`subagent <session id or 8-hex prefix>: model = <id>; purpose = <text>` in any string); a session with none counts under `No declared "
         "model`, and `Off declared model` counts the sessions with a turn in the window on a model other than the declared one." % args.milestone)
    emit("")

    # Self-check
    emit("### Self-check missions")
    emit("")
    sc = aggregate({k: a for k, a in win.items() if clf.of(k[0])["role"] == "self-check"}, lambda sid, m: clf.of(sid)["mission"])
    rows = []
    for mid, a in sorted(sc.items(), key=lambda kv: kv[1].first):
        rows.append(["`%s`" % mid, len(a.sessions), a.turns, a.i, a.o, a.cr, a.cw, usd(a.usd), a.unpriced,
                     ", ".join(m for m, _ in sorted(a.models.items(), key=lambda kv: kv[1])), a.first, a.last,
                     ", ".join(sorted(a.files)) or "-"])
    for r in md_table(["Mission", "Sessions", "Turns", "Input", "Output", "Cache read", "Cache write", "USD-eq", "Unpriced turns",
                       "Models", "First ts", "Last ts", "Source"], rows):
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

    # Owner interventions, idle time and host health
    feature_of = {sid: f for sid, f in clf.feature.items()}
    interventions = usage_idle.intervention_rows(
        progress, pl_label, owner_calls, "sessions/%s/%s.jsonl" % (os.path.basename(os.path.dirname(tpath)), mission_id) if tpath else "transcript not found",
        feature_of, start, end)
    usage_idle.intervention_lines(emit, md_table, interventions)
    events = list(facts.turns) + [r["timestamp"] for _, r in progress if r.get("timestamp") and sec(start) <= sec(r["timestamp"]) <= sec(end)]
    idle_min = usage_idle.idle_lines(emit, md_table, events, facts, pause_rows(progress, pl_label), interventions)
    usage_idle.host_lines(emit, md_table, facts, usage_idle.parse_boottime(args.boottime), now, end if "T" in end else end, idle_min["host"])

    lead_turns = []
    for ts, model, d in sorted(orch_turns, key=lambda x: x[0]):
        a = Acc()
        a.add(ts, d, "")
        lead_turns.append({"ts": ts, "cr": a.cr, "cw": a.cw, "usd": price(prices, model, a) or 0.0})
    in_window = [(r.get("type"), r["timestamp"]) for _, r in progress
                 if r.get("timestamp") and sec(start) <= sec(r["timestamp"]) <= sec(end)]
    usage_rewrites.rewrite_lines(emit, md_table, args.milestone, lead_turns, usd,
                                 boundaries=[ts for t, ts in in_window if t == "worker_completed"],
                                 resumes=[ts for t, ts in in_window if t == "mission_resumed"])

    # User testing
    usage_testing.user_testing_lines(emit, md_table, win, clf, sidecars, args.sessions_dir,
                                     lambda model, a: price(prices, model, a) or 0.0, usd)
    usage_testing.e2e_lines(emit, md_table, args.e2e_dir, repo_root, start, end)

    # Gates
    emit("### Gates")
    emit("")
    gr = gate_rows(args.gate_dir, repo_root, start, end)
    hits = hit_rows(args.gate_dir, start, end)
    label = gate_label(args.gate_dir, repo_root)
    for r in md_table(["Kind", "Gate", "Command", "Count", "Seconds", "Minutes", "First", "Last", "Source"],
                      gates_summary(gr, hits, args.gate_dir, label)):
        emit(r)
    emit("")
    emit("gate runs in the window: %d" % len(gr))
    emit("gate seconds run: %.3f" % sum(g.get("seconds", 0) for g in gr))
    emit("gates served from cache: %d, minutes saved: %.2f" % (len(hits), sum(h["seconds"] for h in hits) / 60))
    emit("")
    emit("Gate window rule: a gate run is in the table when it overlaps the window: `started_at` < `--to` or `finished_at` <= `--to`, "
         "and it did not finish before the window start; a run that started before `--to` and finished after it is in, one that "
         "started at or after `--to` is not (pass a later `--to` to include the integration's own gate runs).")
    emit("")
    emit("Gates run are the records `%s/*/*.json`; gates served from cache are the lines of `%s/hits.log` "
         "by time, and the minutes saved are their stored seconds, the run each hit did not repeat." % (label, label))
    emit("")
    emit("### Gate runs")
    emit("")
    rows = [[g.get("name"), str(g.get("tree", ""))[:12], g.get("exit_code"), "%.3f" % g.get("seconds", 0), g.get("started_at"),
             g.get("finished_at"), "`%s`" % g.get("session"), "`%s`" % g["_path"]] for g in gr]
    for r in md_table(["Gate", "Tree", "Exit", "Seconds", "Started", "Finished", "Session", "Source"], rows):
        emit(r)
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
            f = feats.setdefault(c["feature"], [[], 0, None, None, set()])
            f[0].append(sid)
            f[1] += a.turns
            f[2] = a.first if f[2] is None else min(f[2], a.first)
            f[3] = a.last if f[3] is None else max(f[3], a.last)
            f[4] |= a.files
    feat_usd = collections.Counter()
    for sid, a in by_sess.items():
        c = clf.of(sid)
        if c["role"] in ("worker", "validator") and c["feature"]:
            feat_usd[c["feature"]] += a.usd
    for x in features:
        if x.get("milestone") == args.milestone:
            feats.setdefault(x["id"], [[], 0, None, None, set()])
    budget_of = lambda f: budgets.get(f)  # noqa: E731
    rows = [[f, skills.get(f, "-"), rounds[f], ", ".join("`%s`" % s for s in sorted(v[0])) or "-", v[1],
             "yes" if v[1] > FEATURE_TURN_CAP else "no",
             "-" if budget_of(f) is None else budget_of(f),
             "-" if budget_of(f) is None else ("yes" if v[1] > budget_of(f) else "no"),
             usd(feat_usd[f]), ts_or_dash(v[2]), ts_or_dash(v[3]),
             "`%s`; %s%s" % (", ".join(sorted(v[4])) or "-", pl_label, "; " + ft_label if f in skills else "")]
            for f, v in sorted(feats.items())]
    for r in md_table(["Feature", "Skill", "Rounds", "Sessions", "Turns", "Over %d turns" % FEATURE_TURN_CAP, "Turn budget",
                       "Over budget", "USD-eq", "First ts", "Last ts", "Source"], rows):
        emit(r)
    emit("")
    emit("Rounds are the `worker_started` entries for the feature in `%s`; turns and the first and last timestamps are those of its root "
         "sessions' log turns in the window; the skill and the `turnBudget` are the feature's `skillName` and `turnBudget` in `%s` "
         "(`-` when it has none; the features of the milestone that never ran are listed with 0 turns); USD-eq is the cost of the "
         "feature's sessions and of the subagents they started." % (pl_label, ft_label))
    emit("")

    # Cost per assertion and per feature
    emit("### Cost per assertion and per feature")
    emit("")
    vs_path = args.validation_state or os.path.join(mission_dir, "validation-state.json")
    vs_label = os.path.basename(mission_dir.rstrip("/")) + "/validation-state.json" if not args.validation_state else args.validation_state
    state = None
    if os.path.exists(vs_path):
        with open(vs_path) as f:
            state = json.load(f)
    elif args.validation_state:
        fail("validation state %s not found" % args.validation_state)
    fulfilled = usage_classes.fulfilled_assertions(state, args.milestone) if state else []
    done = [x["id"] for x in features if x.get("milestone") == args.milestone and x.get("status") == "completed"]
    cost_per = lambda n: usd(window_total.usd / n) if n else "-"  # noqa: E731
    for r in md_table(["Unit", "Count", "Window USD-eq", "USD-eq per unit", "Source"], [
            ["assertion fulfilled", len(fulfilled), usd(window_total.usd), cost_per(len(fulfilled)),
             "`%s`%s" % (vs_label, "" if state else " not found")],
            ["feature completed", len(done), usd(window_total.usd), cost_per(len(done)),
             "`%s`%s" % (ft_label, "" if features else " not found")]]):
        emit(r)
    emit("")
    emit("assertions fulfilled in the window: %s" % (", ".join(fulfilled) or "none"))
    emit("")
    emit("Cost per assertion: the window total of Tokens per role divided by the assertions with status `passed` and `validatedAtMilestone` "
         "equal to the milestone (the state file carries no timestamps, so the milestone is its window); cost per feature divides the same "
         "total by the milestone's features with status `completed` in `features.json`. Fix rounds and re-tests are inside the window total.")
    emit("")

    usage_rewrites.allowance_line(emit, args.milestone, window_total.usd, fulfilled, features, args.next_milestone)

    # Mission to date
    emit("### Mission to date by role")
    emit("")
    caps = dict(c.split("=", 1) for c in args.cap)
    td = aggregate(upto, lambda sid, m: role_of(sid))

    def to_date_row(label, a, cap):
        over = "-" if cap is None else ("yes" if a.usd > float(cap) else "no")
        return [label, len(a.sessions), a.turns, a.i, a.o, a.cr, a.cw, usd(a.usd), a.unpriced, cap if cap is not None else "-", over,
                ts_or_dash(a.first), ts_or_dash(a.last), ", ".join(sorted(a.files)) or "-"]

    rows = [to_date_row(role, td.get(role) or Agg(), caps.get(role)) for role in ROLES]
    mission_total = Agg()
    for a in td.values():
        mission_total.merge(a)
    rows.append(to_date_row("total", mission_total, caps.get("total")))
    rows.append(to_date_row("milestone window total", window_total, caps.get("milestone")))
    for r in md_table(["Role", "Sessions", "Turns", "Input", "Output", "Cache read", "Cache write", "USD-eq", "Unpriced turns",
                       "Cap USD-eq", "Over cap", "First ts", "Last ts", "Source"], rows):
        emit(r)
    emit("")
    emit("Role rows and `total` are every log line up to the end of the window, no start bound. `milestone window total` is the "
         "total row of Tokens per role (the window only). Caps are those passed with `--cap role=USD`, `--cap total=USD` and "
         "`--cap milestone=USD`.")
    history = collections.Counter()
    for (_, model), a in upto.items():
        if model not in prices:
            history[model] += a.turns
    if provenance:
        emit("")
        emit("Price provenance: %s" % provenance)
    noted = {}
    for (_, model), a in upto.items():
        if prices.get(model, {}).get("note"):
            n = noted.setdefault(model, [0, 0.0])
            n[0] += a.turns
            n[1] += price(prices, model, a)
    if noted:
        emit("")
        emit("Price notes: %s." % "; ".join("%s (%d turns to date, %s USD-eq): %s" % (m, n[0], usd(n[1]), prices[m]["note"])
                                            for m, n in sorted(noted.items())))
    emit("")
    emit("Models with no price before the window: %s." % (
        "none" if not history else "; ".join("%s (%d turns)" % kv for kv in sorted(history.items())) +
        "; these turns are not in the window, so the run does not stop for them, and they add nothing to USD-eq (counted in `Unpriced turns`)"))
    # Sidecars: lifetime totals of sessions that may still be running, so the one section that can change between runs.
    emit("")
    emit("### Sidecar readings")
    emit("")
    emit("Read from the session sidecars at generation time: lifetime totals of each session, all models, not windowed, and never added to the log rows (D-029). A session still running (the orchestrator, an integrator) moves between two runs with the same `--to`; this is the only section that can, and it is the only source of credits.")
    emit("")
    mt = [mtime_utc(path) for path, _ in sidecars.values()]
    emit("sidecars read: %d, oldest mtime %s, newest mtime %s" % (len(mt), min(mt, default="-"), max(mt, default="-")))
    emit("")
    emit("#### Credits by role (sidecar, cumulative)")
    emit("")
    crole = collections.OrderedDict((r, 0) for r in ROLES)
    for sid in by_sess:
        crole[role_of(sid)] += credit(sid)
    for r in md_table(["Role", "Credits", "Sessions with a sidecar"],
                      [[r, crole[r], sum(1 for sid in by_sess if role_of(sid) == r and sid in sidecars)] for r in ROLES]):
        emit(r)
    emit("")
    emit("#### Sidecar totals per session (cumulative)")
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
                     mtime_utc(path), "`%s/%s.settings.json`" % (os.path.basename(os.path.dirname(path)), sid)])
    for r in md_table(["Session", "Role", "Model", "Effort", "Input", "Output", "Cache read", "Cache write", "Thinking", "Credits",
                       "Active ms", "Modified", "Source"], crow):
        emit(r)
    emit("")
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
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0], epilog=REPRODUCIBILITY,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--milestone", "-m", required=True)
    ap.add_argument("--mission-dir", default=os.environ.get("OBSERVATORY_MISSION_DIR", DEFAULT_MISSION_DIR))
    ap.add_argument("--mission-id", help="orchestrator session id; default the mission directory's name")
    ap.add_argument("--logs-dir", default=os.path.expanduser("~/.factory/logs"))
    ap.add_argument("--sessions-dir", default=os.path.expanduser("~/.factory/sessions"))
    ap.add_argument("--prices", help="price table file; default the newest dated file of --prices-dir, or the one --price-table names")
    ap.add_argument("--prices-dir", default=os.path.join(repo_root, "config", "prices"))
    ap.add_argument("--price-table", help="a dated price file to read instead of the newest: a date (2026-10-22), a file name or a "
                    "path such as the `price_table` of the config")
    ap.add_argument("--validation-state", help="validation-state.json for the cost per assertion; default <mission-dir>/validation-state.json")
    ap.add_argument("--e2e-dir", default=os.path.join(repo_root, "reports", "e2e"),
                    help="reports/e2e: the record.json of each `make e2e` run, listed in the testing section")
    ap.add_argument("--gate-dir", default=os.path.join(repo_root, "reports", "gate"))
    ap.add_argument("--from", dest="start", help="window start (RFC 3339); default the milestone's first worker_started")
    ap.add_argument("--to", help="window end; default the generation time minus 1 s")
    ap.add_argument("--now", help="generation time (tests)")
    ap.add_argument("--boottime", help="the output of `sysctl kern.boottime`, or `unavailable`; default runs the command")
    ap.add_argument("--next-milestone", help="the milestone whose claimed assertions price the weekly allowance share; "
                                             "default the milestone after --milestone in features.json")
    ap.add_argument("--out", help="default reports/usage/<milestone>.md")
    ap.add_argument("--cap", action="append", default=[], metavar="ROLE=USD", help="cap to compare mission-to-date with")
    ap.add_argument("--cross-check", action="store_true", help="compare the session rows with scripts/usage-from-log.sh")
    args = ap.parse_args(argv)
    if not args.prices:
        args.prices = pick_price_table(args.prices_dir, args.price_table)
    if not args.prices:
        sys.exit("usage-report.py: no price table found; pass --prices")
    out = args.out or os.path.join(repo_root, "reports", "usage", args.milestone + ".md")
    body = build(args)
    write_out(out, args.milestone, body)
    print("usage: wrote %s" % out)


if __name__ == "__main__":
    main()
