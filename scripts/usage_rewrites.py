"""Usage report tables for the lead's context rewrites (owner item 19c) and the weekly allowance share line (owner item 7).

Imported by scripts/usage-report.py (same directory); Python 3 standard library only.
"""

import re
import statistics


def context_size(turns):
    """The lead's context: the median nonzero cache read of its turns, 0 when none read the cache.

    Every lead turn re-reads the cached conversation, so its cache read is the context the turn started from; the median
    keeps the few turns that came after a rewrite (small reads) from moving it. Measured, never a constant: it differs
    between milestones and between models.
    """
    reads = [t["cr"] for t in turns if t["cr"]]
    return statistics.median_low(reads) if reads else 0


def rewrites(turns, context):
    """The turns whose cache write exceeds their cache read by at least `context` tokens (empty without a context)."""
    return [t for t in turns if context and t["cw"] - t["cr"] >= context]


BOUNDARY_TARGET = 3


def ratio(turns, boundaries):
    return "%.1f" % (turns / boundaries) if boundaries else "n/a"


def compaction_marker(hits, resumes):
    """The earliest compaction marker as (ts, kind): a progress_log `mission_resumed` line or a lead context rewrite turn.

    A `mission_resumed` line wins a tie with a rewrite at the same time; the orchestrator records its own compaction as a
    `mission_resumed`-style line, and the resumed lead's first turn is then the rewrite.
    """
    found = [(ts, 0, "mission_resumed") for ts in resumes] + [(t["ts"], 1, "lead context rewrite") for t in hits]
    if not found:
        return None
    ts, _, kind = min(found)
    return ts, kind


def single_call_line(calls):
    """The `single-call share` line: lead turns with exactly one tool call over lead turns with at least one (B-176)."""
    with_calls = [n for n in calls if n >= 1]
    if not with_calls:
        return "single-call share: n/a (no orchestrator transcript turn with a tool call in the window)"
    one = sum(1 for n in with_calls if n == 1)
    multi, none = len(with_calls) - one, len(calls) - len(with_calls)
    plural = lambda n: "turn" if n == 1 else "turns"  # noqa: E731
    return ("single-call share: %d / %d = %.1f %% (turns with exactly one tool call over turns with at least one; "
            "%d %s with two or more calls, %d %s with none)" % (one, len(with_calls), 100.0 * one / len(with_calls),
                                                                  multi, plural(multi), none, plural(none)))


def boundary_lines(emit, turns, hits, boundaries, resumes, usd, calls=()):
    """The `orchestrator turns per feature boundary` line, and the before/after split when a compaction marker is in the window.

    `turns` are the lead turns of the rewrite table; `boundaries` are the progress_log times of the `worker_completed` entries
    of the window (the runner's feature returns); `resumes` are the window's `mission_resumed` times; `calls` are the tool-call counts of
    the orchestrator transcript's assistant records in the window.
    """
    emit("orchestrator turns per feature boundary: %d / %d = %s (target %d)"
         % (len(turns), len(boundaries), ratio(len(turns), len(boundaries)), BOUNDARY_TARGET))
    marker = compaction_marker(hits, resumes)
    if marker:
        at, kind = marker
        sides = []
        for pick in (lambda ts: ts < at, lambda ts: ts >= at):
            side = [t for t in turns if pick(t["ts"])]
            n = sum(1 for ts in boundaries if pick(ts))
            per_turn = usd(sum(t["usd"] for t in side) / len(side)) if side else "-"
            sides.append("%d turns / %d boundaries = %s, %s USD-eq per turn" % (len(side), n, ratio(len(side), n), per_turn))
        emit("")
        emit("before compaction: %s; after: %s (marker: %s %s)" % (sides[0], sides[1], kind, at))
    emit("")
    emit(single_call_line(calls))
    emit("")


def rewrite_lines(emit, md_table, milestone, turns, usd, boundaries=(), resumes=(), calls=()):
    """Writes Lead context rewrites, Lead context rewrites per milestone and the summary line.

    `turns` are dicts {ts, cr, cw, usd} of the orchestrator's log turns in the window.
    """
    context = context_size(turns)
    hits = rewrites(turns, context)
    lead_usd = sum(t["usd"] for t in turns)
    hit_usd = sum(t["usd"] for t in hits)
    emit("### Lead context rewrites")
    emit("")
    for r in md_table(["Time", "Cache write", "Cache read", "Cache write minus read", "USD-eq"],
                      [[t["ts"], t["cw"], t["cr"], t["cw"] - t["cr"], usd(t["usd"])] for t in hits]):
        emit(r)
    emit("")
    emit("context size: %d tokens (median nonzero cache read of %d lead turns)" % (context, sum(1 for t in turns if t["cr"])))
    emit("")
    emit("### Lead context rewrites per milestone")
    emit("")
    share = "%.1f" % (100.0 * hit_usd / lead_usd) if lead_usd else "-"
    row = [milestone, len(turns), len(hits), context or "-", sum(t["cw"] for t in hits), sum(t["cr"] for t in hits), usd(hit_usd),
           usd(lead_usd), share]
    for r in md_table(["Milestone", "Lead turns", "Rewrite turns", "Context size", "Cache write", "Cache read", "USD-eq", "Lead USD-eq",
                       "Share of lead cost %"], [row]):
        emit(r)
    emit("")
    if hits:
        emit("context rewrites: %d of %d lead turns, %d cache-write tokens, %s USD-eq (%s %% of the lead's %s)"
             % (len(hits), len(turns), sum(t["cw"] for t in hits), usd(hit_usd), share, usd(lead_usd)))
    else:
        emit("context rewrites: none (no lead turn with a cache read in the window)" if not context
             else "context rewrites: none in %d lead turns" % len(turns))
    emit("")
    boundary_lines(emit, turns, hits, boundaries, resumes, usd, calls)
    emit("A rewrite is a lead turn whose cache-write tokens exceed its cache-read tokens by at least the context size: the turn "
         "wrote a new cache of roughly the whole conversation instead of extending the old one (a compaction, a cache expiry "
         "after a long wait, or a resume). The context size is measured per window from the lead's own turns, not fixed; the "
         "USD-eq of a rewrite turn is the whole turn (its input, output, cache read and cache write), and the lead is the "
         "orchestrator role. The tokens are the log's `cachedTokensWritten` and `cacheReadInputTokens`.")
    emit("")
    emit("The turns per feature boundary are the lead turns of the window divided by the `worker_completed` entries of the "
         "mission's progress_log.jsonl in the window (one per worker hand-off returned to the orchestrator; `worker_failed` is "
         "not one). The before/after line appears only when the window holds a compaction marker, a `mission_resumed` entry of "
         "the progress_log or a lead context rewrite above, and splits at the earliest one.")
    emit("")
    emit("The single-call share counts the assistant records of the orchestrator transcript in the window: a turn with exactly one `tool_use` "
         "block (a Script is one call) over the turns with at least one; a turn with none is a closing text and counts in neither, and "
         "parallel calls in one turn are what the share is meant to raise a call to.")
    emit("")


def claimed_assertions(features, milestone):
    """Unique assertion ids that the features of `milestone` claim in `fulfills`."""
    ids = set()
    for x in features:
        if x.get("milestone") == milestone:
            ids.update(i for i in x.get("fulfills") or () if isinstance(i, str))
    return ids


def next_milestone(features, milestone):
    """The lowest milestone `m<K>` of features.json above `milestone` `m<N>` (K > N), or None.

    features.json is not in milestone order (the pending milestones are listed first), so the order is the number.
    """
    have = re.fullmatch(r"m(\d+)", milestone or "")
    later = [int(m.group(1)) for x in features for m in [re.fullmatch(r"m(\d+)", x.get("milestone") or "")]
             if m and have and int(m.group(1)) > int(have.group(1))]
    return "m%d" % min(later) if later else None


def allowance_line(emit, milestone, window_usd, fulfilled, features, named=None):
    """The `weekly allowance share` line: this window's cost per assertion times the next milestone's claimed assertions."""
    nxt = named or next_milestone(features, milestone)
    if not fulfilled:
        emit("weekly allowance share: unavailable (no assertion fulfilled in %s)" % milestone)
    elif not nxt:
        emit("weekly allowance share: none (no milestone after %s in features.json)" % milestone)
    else:
        per = window_usd / len(fulfilled)
        claims = len(claimed_assertions(features, nxt))
        emit("weekly allowance share: %s claims %d assertions x %.2f USD-eq per assertion (%s window, %d fulfilled) = %.2f USD-eq"
             % (nxt, claims, per, milestone, len(fulfilled), per * claims))
    emit("")
    emit("The orchestrator states the weekly allowance share before it dispatches the next milestone: the cost per assertion of this "
         "window (Cost per assertion and per feature) times the assertions the next milestone's features claim in `fulfills`. It is "
         "a forecast of the share, not a cap; the weekly allowance is the owner's to manage (D-038).")
    emit("")
