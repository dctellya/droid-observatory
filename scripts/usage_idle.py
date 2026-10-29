"""Usage report tables for idle time by cause, host health and owner interventions (owner items 6, 7, 8 of the M3 boundary).

Imported by scripts/usage-report.py (same directory); Python 3 standard library only. Everything here is read from the
Droid log lines the report scans, progress_log.jsonl and the orchestrator transcript; nothing is estimated.
"""

import bisect
import collections
import json
import re
import subprocess
from datetime import datetime, timezone

# Owner item 6: a gap over 15 minutes between two recorded events is idle time.
IDLE_GAP_S = 900
# A failure line is logged a few seconds after the last good turn and an answer a few seconds before the next one, so the
# lines that explain a gap are read from this far outside its bounds.
EVIDENCE_MARGIN_S = 60
# Owner item 8: macOS stops opening TCP connections after 2**32 ms of uptime, 49.7 days.
TCP_CLOCK_LIMIT_DAYS = 49.7
CAUSES = ("usage limit", "host", "owner", "unexplained")
# errno codes that a network or port fault leaves in the Droid log (25-oct 10:12 to 11:18 local: exhausted ports).
HOST_FAULT_CODES = ("ECONNREFUSED", "EADDRNOTAVAIL", "ENOBUFS", "ENETDOWN", "ENETUNREACH")
USAGE_REASON = "paymentRequired"
RESETS_IN = re.compile(r"resets in ([^)]+)\)")
BOOTTIME_SEC = re.compile(r"sec\s*=\s*(\d+)")


class LogFacts:
    """What one scan of the Droid log leaves for these tables, window lines only."""

    def __init__(self):
        self.turns = []
        self.calls = []
        self.faults = []

    def note(self, ts, line):
        route = "[LLM] Chat route failure" in line
        retried = '"retried":true' in line
        rotated = '"providerRotated":true' in line
        if route or retried or rotated:
            ctx = {}
            i = line.find(" | Context: ")
            try:
                ctx = json.loads(line[i + 12:]) if i >= 0 else {}
            except ValueError:
                pass
            err = ctx.get("error") or {}
            self.calls.append({"ts": ts, "amid": ctx.get("assistantMessageId") or ts, "route": route, "reason": ctx.get("reason") or "-",
                               "retried": retried, "rotated": rotated,
                               "message": str(err.get("message", "")) if isinstance(err, dict) else ""})
        for code in HOST_FAULT_CODES:
            if code in line:
                self.faults.append((ts, code))


def to_dt(ts):
    return datetime.fromisoformat(ts)


def fmt_ms(dt):
    return dt.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.") + "%03dZ" % (dt.microsecond // 1000)


def parse_boottime(arg, run=subprocess.run):
    """(text to print, boot instant or None) from `--boottime <text or epoch>` or from `sysctl kern.boottime` itself."""
    text = arg
    if text is None:
        try:
            proc = run(["sysctl", "kern.boottime"], capture_output=True, text=True, timeout=10)
            text = proc.stdout.strip() if proc.returncode == 0 else "unavailable"
        except (OSError, subprocess.SubprocessError):
            text = "unavailable"
    m = BOOTTIME_SEC.search(text)
    if m:
        return text, datetime.fromtimestamp(int(m.group(1)), timezone.utc)
    if text.strip().isdigit():
        return text, datetime.fromtimestamp(int(text), timezone.utc)
    return "unavailable", None


def idle_gaps(event_ts):
    """[(start, end, seconds)] between consecutive events that are more than IDLE_GAP_S apart; event_ts are ISO strings."""
    pts = sorted({to_dt(t) for t in event_ts})
    return [(a, b, (b - a).total_seconds()) for a, b in zip(pts, pts[1:]) if (b - a).total_seconds() > IDLE_GAP_S]


def near(ts, a, b):
    t = to_dt(ts) if isinstance(ts, str) else ts
    return (a.timestamp() - EVIDENCE_MARGIN_S) <= t.timestamp() <= (b.timestamp() + EVIDENCE_MARGIN_S)


def overlaps(p0, p1, a, b):
    return p0 <= b and (p1 is None or p1 >= a)


def classify_gap(a, b, facts, pauses, interventions):
    """(cause, evidence) of one gap: a usage limit, then the host, then the owner; nothing recorded is `unexplained`."""
    pay = [c for c in facts.calls if c["route"] and c["reason"] == USAGE_REASON and near(c["ts"], a, b)]
    throttled = [p for p in pauses if p["throttled"] and overlaps(to_dt(p["pause"]), p["until"] and to_dt(p["until"]), a, b)]
    if pay or throttled:
        bits = ["%s %s %s" % ("mission_paused", p["pause"], p["reason"]) for p in throttled]
        if pay:
            resets = RESETS_IN.search(pay[0]["message"])
            bits.append("Chat route failure %s %d%s" % (USAGE_REASON, len(pay), " (resets in %s)" % resets.group(1) if resets else ""))
        return "usage limit", "; ".join(bits)
    codes = collections.Counter(code for ts, code in facts.faults if near(ts, a, b))
    other = collections.Counter(c["reason"] for c in facts.calls if c["route"] and c["reason"] != USAGE_REASON and near(c["ts"], a, b))
    if codes or other:
        bits = ["%s %d" % kv for kv in sorted(codes.items())] + ["Chat route failure %s %d" % kv for kv in sorted(other.items())]
        return "host", ", ".join(bits)
    owner_pauses = [p for p in pauses if not p["throttled"] and overlaps(to_dt(p["pause"]), p["until"] and to_dt(p["until"]), a, b)]
    ints = [i for i in interventions if near(i["time"], a, b)]
    if owner_pauses or ints:
        bits = ["mission_paused %s to %s" % (p["pause"], p["until"] or "open") for p in owner_pauses]
        bits += ["%s %s" % (i["kind"], i["time"]) for i in ints]
        return "owner", "; ".join(bits)
    return "unexplained", "none"


def idle_lines(emit, md_table, event_ts, facts, pauses, interventions):
    """Writes Idle time by cause and Idle gaps; returns the minutes idle by cause."""
    gaps = [(a, b, s) + classify_gap(a, b, facts, pauses, interventions) for a, b, s in idle_gaps(event_ts)]
    minutes = collections.OrderedDict((c, [0, 0.0]) for c in CAUSES)
    for _, _, s, cause, _ in gaps:
        minutes[cause][0] += 1
        minutes[cause][1] += s / 60
    emit("### Idle time by cause")
    emit("")
    rows = [[c, n, "%.2f" % m] for c, (n, m) in minutes.items()]
    rows.append(["total", sum(n for n, _ in minutes.values()), "%.2f" % sum(m for _, m in minutes.values())])
    for r in md_table(["Cause", "Gaps", "Minutes"], rows):
        emit(r)
    emit("")
    unexplained = [g for g in gaps if g[3] == "unexplained"]
    if not unexplained:
        emit("unexplained: none")
    for a, b, s, _, _ in unexplained:
        emit("unexplained gap: %s to %s (%.2f min)" % (fmt_ms(a), fmt_ms(b), s / 60))
    emit("")
    emit("A gap is the time between two consecutive events inside the window: a model turn in the Droid log (any session) or a record "
         "of progress_log.jsonl. It is idle time when it lasts more than %d minutes. Its cause is the first that has evidence within %d s "
         "of the gap: `usage limit` (a throttled pause in progress_log.jsonl, or a `[LLM] Chat route failure` with reason `%s`), `host` "
         "(%s, or a route failure with another reason, in the Droid log), `owner` (an owner pause, `mission_paused` without a throttle "
         "reason, overlapping the gap, or an owner intervention at it); a gap with none of these is `unexplained`."
         % (IDLE_GAP_S // 60, EVIDENCE_MARGIN_S, USAGE_REASON, ", ".join("`%s`" % c for c in HOST_FAULT_CODES)))
    emit("")
    emit("### Idle gaps")
    emit("")
    for r in md_table(["Start", "End", "Minutes", "Cause", "Evidence"],
                      [[fmt_ms(a), fmt_ms(b), "%.2f" % (s / 60), cause, ev] for a, b, s, cause, ev in gaps]):
        emit(r)
    emit("")
    return {c: m for c, (_, m) in minutes.items()}


def call_rows(facts):
    """Rows of Model call faults, and the distinct calls that stalled or were retried with the usage limit left out."""
    groups = collections.OrderedDict()

    def add(kind, ts, call):
        g = groups.setdefault(kind, {"lines": 0, "calls": set(), "ts": []})
        g["lines"] += 1
        if call is not None:
            g["calls"].add(call)
        g["ts"].append(ts)

    for c in facts.calls:
        if c["route"]:
            add("Chat route failure: %s" % c["reason"], c["ts"], c["amid"])
    for key, kind in (("retried", "retried: true"), ("rotated", "providerRotated: true")):
        for c in facts.calls:
            if c[key]:
                add(kind, c["ts"], c["amid"])
    for ts, code in facts.faults:
        add(code, ts, None)
    rows = [[kind, g["lines"], len(g["calls"]) if g["calls"] else "-", min(g["ts"]), max(g["ts"])] for kind, g in groups.items()]
    stalled = {c["amid"] for c in facts.calls if not (c["route"] and c["reason"] == USAGE_REASON)}
    return rows, len(stalled)


def host_lines(emit, md_table, facts, boottime, now, end, host_idle_min):
    """Writes Host health and Model call faults; returns nothing, the verdict is one printed line."""
    text, boot = boottime
    end_dt = to_dt(end)
    up_now = (now - boot).total_seconds() / 86400 if boot else None
    up_end = (end_dt - boot).total_seconds() / 86400 if boot and boot <= end_dt else None
    emit("### Host health")
    emit("")
    emit("sysctl kern.boottime: %s" % text)
    emit("")
    rows = [["Boot time (UTC)", fmt_ms(boot) if boot else "unavailable"],
            ["Uptime at generation", "%.2f days" % up_now if boot else "unavailable"],
            ["Uptime at the window end", "%.2f days" % up_end if up_end is not None else
             ("booted after the window end: the host of the window was an earlier boot, its uptime is not recorded" if boot else "unavailable")],
            ["TCP clock limit", "%s days" % TCP_CLOCK_LIMIT_DAYS],
            ["Days to the limit at generation", ("%.2f" % (TCP_CLOCK_LIMIT_DAYS - up_now) if up_now < TCP_CLOCK_LIMIT_DAYS else "past the limit")
             if boot else "unavailable"]]
    for r in md_table(["Check", "Value"], rows):
        emit(r)
    emit("")
    emit("### Model call faults")
    emit("")
    call_table, stalled = call_rows(facts)
    for r in md_table(["Kind", "Lines", "Calls", "First", "Last"], call_table):
        emit(r)
    emit("")
    emit("stalled or retried model calls, usage limit excluded: %d" % stalled)
    emit("")
    over = up_end is not None and up_end >= TCP_CLOCK_LIMIT_DAYS
    parts = ["%.2f min idle by host faults" % host_idle_min if host_idle_min > 0 else "no idle time by host faults"]
    if up_end is not None:
        parts.append("uptime %.2f days is past %s days" % (up_end, TCP_CLOCK_LIMIT_DAYS) if over
                     else "uptime %.2f days (limit %s)" % (up_end, TCP_CLOCK_LIMIT_DAYS))
    parts.append("%d stalled or retried calls" % stalled)
    emit("host verdict: %s: %s" % ("inflated" if host_idle_min > 0 or over else "did not inflate the numbers", "; ".join(parts)))
    emit("")
    emit("Model call faults are the window's Droid log lines `[LLM] Chat route failure` (by reason), lines with `\"retried\":true` or "
         "`\"providerRotated\":true`, and the errno codes %s; `Calls` counts distinct `assistantMessageId`. A usage-limit failure "
         "(`%s`) is the plan's stop, not the host's, so it is shown but left out of the stalled or retried count. The verdict says "
         "`inflated` when the window has idle time by host faults or the host's uptime at the window end passed the macOS TCP clock "
         "limit (2**32 ms, %s days), and `did not inflate` otherwise." % (", ".join(HOST_FAULT_CODES), USAGE_REASON, TCP_CLOCK_LIMIT_DAYS))
    emit("")


def text_of(content):
    if isinstance(content, str):
        return content
    return " ".join(b.get("text", "") for b in content or () if isinstance(b, dict) and b.get("type") == "text")


def short(text, n=160):
    one = " ".join(str(text).split())
    return one if len(one) <= n else one[: n - 1] + "…"


def intervention_rows(progress, pl_label, transcript, transcript_label, feature_of_session, start, end):
    """Owner messages that unblocked the run, in time order: resumes (progress_log), AskUser answers and messageToWorker (transcript)."""
    starts = sorted((to_dt(r["timestamp"]), r.get("featureId") or "-") for _, r in progress if r.get("type") == "worker_started")
    start_times = [t for t, _ in starts]

    def next_feature(t):
        i = bisect.bisect_left(start_times, t)
        return starts[i][1] if i < len(starts) else "-"

    rows = []
    for ln, r in progress:
        if r.get("type") == "mission_resumed":
            sid = r.get("resumeWorkerSessionId")
            rows.append({"time": r["timestamp"], "kind": "mission_resumed",
                         "feature": feature_of_session.get(sid) or next_feature(to_dt(r["timestamp"])),
                         "detail": "resumes session %s" % sid[:8] if sid else "-", "source": "%s:%d" % (pl_label, ln)})
    for tid, (ts, question) in (transcript.get("asks") or {}).items():
        answer = (transcript.get("answers") or {}).get(tid)
        if answer:
            rows.append({"time": answer[0], "kind": "AskUser answer", "feature": next_feature(to_dt(answer[0])),
                         "detail": short(answer[1]), "source": "%s (%s)" % (transcript_label, tid)})
    for tid, (ts, sid, message) in (transcript.get("messages") or {}).items():
        rows.append({"time": ts, "kind": "messageToWorker", "feature": feature_of_session.get(sid) or next_feature(to_dt(ts)),
                     "detail": short(message), "source": "%s (%s)" % (transcript_label, tid)})
    rows = [r for r in rows if to_dt(start) <= to_dt(r["time"]) <= to_dt(end)]
    rows.sort(key=lambda r: to_dt(r["time"]))
    return rows


def intervention_lines(emit, md_table, rows):
    emit("### Owner interventions")
    emit("")
    for r in md_table(["Time", "Kind", "Feature unblocked", "Detail", "Source"],
                      [[r["time"], r["kind"], r["feature"], r["detail"], "`%s`" % r["source"]] for r in rows]):
        emit(r)
    emit("")
    kinds = collections.Counter(r["kind"] for r in rows)
    order = ("mission_resumed", "AskUser answer", "messageToWorker")
    emit("owner interventions: %d (%s)" % (len(rows), ", ".join("%s %d" % (k, kinds[k]) for k in order if kinds[k]) or "none"))
    emit("")
    emit("Owner messages that unblocked the run: `mission_resumed` records of progress_log.jsonl (the feature is that of the session "
         "it resumes, else of the next `worker_started`), the answer to each `AskUser` call and each `StartMissionRun` carrying a "
         "`messageToWorker`, both read from the orchestrator transcript (the feature is that of the session the message resumes, else "
         "of the next `worker_started`).")
    emit("")
