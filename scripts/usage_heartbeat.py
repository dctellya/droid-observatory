"""Usage report: the orchestrator's heartbeat turns apart from its boundary turns (owner item 1 of the M5 boundary, inbox 1350).

Imported by scripts/usage-report.py (same directory); Python 3 standard library only.

The detector is read from the orchestrator transcript (the same calls `build-log/<date>/<orchestrator session>.jsonl` logs, where the
Script source is cut to 200 characters, so the report reads the full source from the transcript): a heartbeat is a Script that sleeps
and then reads the inbox, the progress file and the handoffs, and is the turn's only call.
"""

import collections
import re

# Rule E of the mission loop: 240 s until inbox 0030 (2026-10-30 00:30Z) amended it to 200 s, so the transcript holds both.
HEARTBEAT_SLEEPS_MS = (200000, 240000)
HEARTBEAT_READS = ("inbox", "progress", "handoffs")
SLEEP = re.compile(r"setTimeout\(\s*\w+\s*,\s*(\d+)\s*\)")
KINDS = ("heartbeat", "boundary", "unmatched")


def why_not(rec):
    """None when the turn is a heartbeat, else the first condition it fails."""
    if len(rec["top"]) != 1:
        return "%d tool calls" % len(rec["top"])
    if rec["top"][0] != "Script":
        return "its only call is %s, not a Script" % rec["top"][0]
    source = rec["scripts"][0] if rec["scripts"] else ""
    sleeps = [int(n) for n in SLEEP.findall(source)]
    if not any(n in HEARTBEAT_SLEEPS_MS for n in sleeps):
        return "no sleep of %s ms" % " or ".join(str(n) for n in HEARTBEAT_SLEEPS_MS)
    missing = [w for w in HEARTBEAT_READS if w not in source.lower()]
    return "no read of %s" % ", ".join(missing) if missing else None


def is_heartbeat(rec):
    return why_not(rec) is None


def excerpt(rec, lines=3):
    return " / ".join(l.strip() for l in (rec["scripts"][0] if rec["scripts"] else "").splitlines()[:lines])


def heartbeat_lines(emit, md_table, usd, turns, completions):
    """`turns` is [(ts, Acc, usd, rec or None)] of the lead's log turns in the window; `completions` the worker_completed count."""
    totals = collections.OrderedDict((k, [0, 0, 0, 0, 0, 0.0]) for k in KINDS)
    for _, a, cost, rec in turns:
        kind = "unmatched" if rec is None else "heartbeat" if is_heartbeat(rec) else "boundary"
        t = totals[kind]
        t[0] += a.turns
        t[1] += a.i
        t[2] += a.o
        t[3] += a.cr
        t[4] += a.cw
        t[5] += cost
    n = sum(t[0] for t in totals.values())
    rows = [[k, t[0], t[1], t[2], t[3], t[4], usd(t[5]), "%.1f" % (100 * t[0] / n if n else 0)] for k, t in totals.items()]
    rows.append(["total", n] + [sum(t[i] for t in totals.values()) for i in (1, 2, 3, 4)] +
                [usd(sum(t[5] for t in totals.values())), "100.0" if n else "0.0"])
    emit("### Orchestrator heartbeat and boundary turns")
    emit("")
    for r in md_table(["Kind", "Turns", "Input", "Output", "Cache read", "Cache write", "USD-eq", "Share of turns %"], rows):
        emit(r)
    emit("")
    boundary = totals["boundary"][0]
    emit("boundary turns per feature boundary: %s" % ("%d / %d = %.1f" % (boundary, completions, boundary / completions)
                                                      if completions else "%d / 0 = -" % boundary))
    emit("")
    emit("heartbeat detector: an orchestrator turn whose only tool call is a Script whose source contains a sleep of %s and reads the %s "
         "(`setTimeout(r, N)` with N in %s; the reads are tested on the lower-cased source). The sleep is %s: Rule E began at 240 s and "
         "inbox 0030 (2026-10-30 00:30Z) moved it to 200 s, so the transcript holds both. A turn that matches is a `heartbeat`; every other "
         "orchestrator turn joined to the transcript is a `boundary` turn (a decision, a dispatch, a hand-off, a report); a log turn with "
         "no transcript record within the matching tolerance is `unmatched` and in neither."
         % (" or ".join("%d ms (%d s)" % (n, n // 1000) for n in HEARTBEAT_SLEEPS_MS), ", ".join(HEARTBEAT_READS),
            ", ".join(str(n) for n in HEARTBEAT_SLEEPS_MS), " / ".join("%d ms" % n for n in HEARTBEAT_SLEEPS_MS)))
    emit("")
    recs = sorted((r for _, _, _, r in turns if r is not None), key=lambda r: r["t"])
    match = next((r for r in recs if is_heartbeat(r)), None)
    miss = next((r for r in recs if not is_heartbeat(r) and "setTimeout" in " ".join(r["scripts"])), None) or \
        next((r for r in recs if not is_heartbeat(r)), None)
    emit("heartbeat example (matched): %s %s" % (match["ts"], excerpt(match)) if match else "heartbeat example (matched): none in the window")
    emit("")
    emit("boundary example (not matched): %s %s (%s)" % (miss["ts"], excerpt(miss) or "no Script source", why_not(miss))
         if miss else "boundary example (not matched): none in the window")
    emit("")
