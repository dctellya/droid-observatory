"""Usage report tables for the testing section: user testing (cost per round and per flow group) and the e2e runs
(`flaked` is a line of its own, never a pass).

Imported by scripts/usage-report.py (same directory); Python 3 standard library only.
"""

import collections
import glob
import json
import os

USER_TESTING_FEATURE = "user-testing-validator-"
NO_TASK = "(no Task description found)"
MAX_DEPTH = 8


def task_descriptions(path):
    """{tool_use id: description} of the Task calls in one session transcript."""
    out = {}
    if not path:
        return out
    with open(path, errors="replace") as f:
        for line in f:
            if '"name":"Task"' not in line:
                continue
            try:
                r = json.loads(line)
            except ValueError:
                continue
            content = (r.get("message") or {}).get("content")
            for b in content if isinstance(content, list) else ():
                if b.get("type") == "tool_use" and b.get("name") == "Task":
                    out[b.get("id")] = str((b.get("input") or {}).get("description") or "")
    return out


def calling(sidecars, sid):
    """(callingSessionId, callingToolUseId) of a subagent session, or (None, None)."""
    for t in (sidecars.get(sid, (None, {}))[1].get("tags")) or []:
        if isinstance(t, dict) and t.get("name") == "subagent":
            meta = t.get("metadata") or {}
            return meta.get("callingSessionId"), meta.get("callingToolUseId")
    return None, None


def chain_to_root(sidecars, sid):
    """[sid, its caller, ..., root session]."""
    chain = [sid]
    while len(chain) < MAX_DEPTH:
        caller, _ = calling(sidecars, chain[-1])
        if not caller or caller in chain:
            break
        chain.append(caller)
    return chain


def user_testing_lines(emit, md_table, win, clf, sidecars, sessions_dir, cost, usd):
    """Writes User testing rounds and User testing flow groups from the window's per-(session, model) sums."""
    per_session = collections.defaultdict(lambda: {"turns": 0, "usd": 0.0, "first": None, "last": None, "models": set()})
    for (sid, model), a in win.items():
        s = per_session[sid]
        s["turns"] += a.turns
        s["usd"] += cost(model, a)
        s["models"].add(model)
        s["first"] = a.first if s["first"] is None else min(s["first"], a.first)
        s["last"] = a.last if s["last"] is None else max(s["last"], a.last)
    rounds = collections.OrderedDict()
    for sid in per_session:
        chain = chain_to_root(sidecars, sid)
        root = chain[-1]
        c = clf.of(root)
        if c["role"] == "validator" and c["kind"] == "root" and (c["feature"] or "").startswith(USER_TESTING_FEATURE):
            rounds.setdefault(root, []).append((sid, chain))
    ordered = sorted(rounds.items(), key=lambda kv: min(per_session[s]["first"] for s, _ in kv[1]))
    round_rows, group_rows = [], []
    total = {"validator": 0.0, "subagents": 0.0}
    n_groups = 0
    for number, (root, members) in enumerate(ordered, 1):
        descriptions = task_descriptions(next(iter(glob.glob(os.path.join(sessions_dir, "*", root + ".jsonl"))), None))
        groups = collections.OrderedDict()
        validator = subagents = 0.0
        v_turns = 0
        firsts, lasts = [], []
        for sid, chain in sorted(members, key=lambda m: per_session[m[0]]["first"]):
            s = per_session[sid]
            firsts.append(s["first"])
            lasts.append(s["last"])
            if sid == root:
                validator += s["usd"]
                v_turns += s["turns"]
                continue
            subagents += s["usd"]
            top = chain[-2]
            label = descriptions.get(calling(sidecars, top)[1]) or NO_TASK
            g = groups.setdefault(label, {"sessions": 0, "turns": 0, "usd": 0.0, "models": set(), "first": s["first"], "last": s["last"]})
            g["sessions"] += 1
            g["turns"] += s["turns"]
            g["usd"] += s["usd"]
            g["models"] |= s["models"]
            g["first"], g["last"] = min(g["first"], s["first"]), max(g["last"], s["last"])
        n_groups += len(groups)
        total["validator"] += validator
        total["subagents"] += subagents
        round_rows.append([number, "`%s`" % root, len(groups), sum(g["sessions"] for g in groups.values()), v_turns, usd(validator),
                           usd(subagents), usd(validator + subagents), min(firsts), max(lasts)])
        for label, g in groups.items():
            group_rows.append([number, label, g["sessions"], g["turns"], usd(g["usd"]), ", ".join(sorted(g["models"])), g["first"], g["last"]])
    emit("### User testing rounds")
    emit("")
    for r in md_table(["Round", "Session", "Flow groups", "Subagent sessions", "Validator turns", "Validator USD-eq", "Subagent USD-eq",
                       "Total USD-eq", "First ts", "Last ts"], round_rows):
        emit(r)
    emit("")
    emit("### User testing flow groups")
    emit("")
    for r in md_table(["Round", "Group", "Subagent sessions", "Turns", "USD-eq", "Models", "First ts", "Last ts"], group_rows):
        emit(r)
    emit("")
    if ordered:
        emit("user testing: %d rounds, %d flow groups, %s USD-eq (validator sessions %s, subagents %s)" % (
            len(ordered), n_groups, usd(total["validator"] + total["subagents"]), usd(total["validator"]), usd(total["subagents"])))
    else:
        emit("user testing: none")
    emit("")
    emit("A round is a root session of the feature `user-testing-validator-*` with turns in the window, numbered by its first turn; "
         "its cost is the session's own turns plus every subagent session under it (nested ones too). A flow group is a subagent the "
         "validator started with the Task tool: its `callingToolUseId` in the sidecar is matched to the `description` of that Task call in "
         "the validator's transcript, and the subagents it started count in its group. Scrutiny validators are not user testing.")
    emit("")


def e2e_records(e2e_dir, repo_root, start, end):
    """Every reports/e2e/<date>/<HHMMSS>/record.json (scripts/e2e_run.py; also the flat <run>/record.json of the runs before
    run stamps) that overlaps the window, oldest first. `latest` is a symlink to one of them and is not counted again."""
    rows = []
    seen = set()
    for path in sorted(glob.glob(os.path.join(e2e_dir, "*", "*", "record.json")) + glob.glob(os.path.join(e2e_dir, "*", "record.json"))):
        if os.path.islink(os.path.dirname(path)) or os.path.realpath(path) in seen:
            continue
        seen.add(os.path.realpath(path))
        try:
            with open(path) as f:
                r = json.load(f)
        except (ValueError, OSError):
            continue
        st = r.get("started_at") or r.get("finished_at")
        fin = r.get("finished_at") or st
        if not st or not ((st[:19] < end[:19] or fin[:19] <= end[:19]) and fin[:19] >= start[:19]):
            continue
        r["_path"] = os.path.relpath(path, repo_root) if path.startswith(repo_root) else os.path.join(
            os.path.basename(e2e_dir.rstrip("/")), os.path.relpath(path, e2e_dir))
        rows.append(r)
    rows.sort(key=lambda r: (r.get("started_at") or "", r.get("run") or ""))
    return rows


def rerun_seconds(record):
    attempts = record.get("attempts") or []
    if record.get("rerun_seconds") is not None:
        return float(record["rerun_seconds"])
    return float(attempts[1]["seconds"]) if len(attempts) > 1 else 0.0


def e2e_lines(emit, md_table, e2e_dir, repo_root, start, end):
    """Writes the E2E runs table, one `e2e flake:` line per flaked run and the `e2e runs:` summary."""
    records = e2e_records(e2e_dir, repo_root, start, end)
    emit("### E2E runs")
    emit("")
    rows = []
    for r in records:
        attempts = r.get("attempts") or [{}]
        specs = r.get("flaked_specs") or r.get("failed_specs") or []
        rows.append([r.get("run"), r.get("outcome"), ", ".join(specs) or "-", "%.2f" % float(attempts[0].get("seconds", 0)),
                     "%.2f" % rerun_seconds(r) if len(attempts) > 1 else "-", r.get("started_at"), "`%s`" % r["_path"]])
    for line in md_table(["Run", "Outcome", "Specs", "First attempt s", "Rerun s", "Started", "Source"], rows):
        emit(line)
    emit("")
    for r in records:
        if r.get("outcome") == "flaked":
            emit("e2e flake: %s: %s; rerun %.2f s (first attempt %.2f s)" % (
                r.get("run"), ", ".join(r.get("flaked_specs") or []), rerun_seconds(r), float((r["attempts"][0]).get("seconds", 0))))
    count = collections.Counter(r.get("outcome") for r in records)
    if records:
        emit("e2e runs: %d, passed %d, flaked %d, failed %d; rerun seconds: %.2f" % (
            len(records), count["passed"], count["flaked"], count["failed"], sum(rerun_seconds(r) for r in records)))
    else:
        emit("e2e runs: none")
    emit("")
    emit("`flaked` is a first attempt that failed and one rerun that passed: the run is green and counted as a flake, never as a "
         "pass; `failed` is a rerun that failed too (or a failure that named no spec). The runs are the records "
         "`reports/e2e/<date>/<HHMMSS>/record.json` that overlap the window (every run directory, so a flake stays counted after a later "
         "green run); the evidence of the first failure is in `reports/e2e/<date>/<HHMMSS>/first-failure/`, which is git-ignored and goes with the `e2e-reports` artifact of the CI run.")
    emit("")
