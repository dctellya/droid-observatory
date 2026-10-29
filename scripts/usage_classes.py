"""Usage report tables that split cost by token class and read the declarations beside the log.

Imported by scripts/usage-report.py (same directory); Python 3 standard library only.
"""

import glob
import json
import os
import re

# A subagent declares its model and purpose in a handoff or a synthesis file, either as a `subagents` list or a
# `subagentSessions` object, or as one line of text in any string: `subagent <session id or 8-hex prefix>: model = <id>; purpose = <text>`.
DECLARATION_LINE = re.compile(r"subagent\s+`?([0-9a-f]{8}(?:-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})?)`?\s*:\s*"
                              r"model\s*=\s*([\w.\-]+)\s*;\s*purpose\s*=\s*([^\n]+)")

# (name, Acc attribute, price-table rate key), in the order cost follows the class: fresh input, cache write, cache read, output.
TOKEN_CLASSES = (("input", "i", "input"), ("cache write", "cw", "cache_write"), ("cache read", "cr", "cache_read"), ("output", "o", "output"))


# The mission runner rewrites features.json with its own schema and drops `turnBudget`; the budget then stays in the description text.
BUDGET_TEXT = re.compile(r"\bturnBudget\s+(\d+)\b")


def turn_budget(feature):
    """The feature's `turnBudget` key, else the N of `turnBudget <N>` in its description, else None."""
    if feature.get("turnBudget") is not None:
        return feature["turnBudget"]
    m = BUDGET_TEXT.search(feature.get("description") or "")
    return int(m.group(1)) if m else None


def new_totals():
    return {name: [0, 0.0] for name, _, _ in TOKEN_CLASSES}


def add_totals(totals, acc, rates):
    """Add one Acc priced at `rates` (per million tokens by class) to the per-class [tokens, usd] totals."""
    for name, attr, key in TOKEN_CLASSES:
        n = getattr(acc, attr)
        totals[name][0] += n
        totals[name][1] += n * rates[key] / 1e6


def share(part, whole):
    return "%.1f" % (100.0 * part / whole) if whole else "-"


def class_rows(scope, totals):
    tokens = sum(t for t, _ in totals.values())
    cost = sum(c for _, c in totals.values())
    return [[scope, name, n, share(n, tokens), "%.2f" % c, share(c, cost)] for name, (n, c) in totals.items()]


def fulfilled_assertions(state, milestone):
    """Ids of the assertions that moved to passed at `milestone` (validation-state.json has no timestamps, only the milestone)."""
    return sorted(k for k, v in (state.get("assertions") or {}).items()
                  if isinstance(v, dict) and v.get("status") == "passed" and v.get("validatedAtMilestone") == milestone)


def _declared(node, source, out):
    if isinstance(node, dict):
        for key in ("subagents", "subagentSessions"):
            group = node.get(key)
            items = group.items() if isinstance(group, dict) else ((d.get("sessionId"), d) for d in group or () if isinstance(d, dict))
            for sid, d in items:
                if isinstance(d, dict) and sid and d.get("model"):
                    out[str(sid)] = (str(d["model"]), str(d.get("purpose") or "-"), source)
        for v in node.values():
            _declared(v, source, out)
    elif isinstance(node, list):
        for v in node:
            _declared(v, source, out)
    elif isinstance(node, str):
        for sid, model, purpose in DECLARATION_LINE.findall(node):
            out[sid] = (model, purpose.strip(), source)


def read_declarations(mission_dir, milestone):
    """{session id or prefix: (model, purpose, file)} from handoffs/*.json and validation/<milestone>/**/synthesis*.json."""
    paths = sorted(glob.glob(os.path.join(mission_dir, "handoffs", "*.json"))
                   + glob.glob(os.path.join(mission_dir, "validation", milestone, "**", "synthesis*.json"), recursive=True))
    out = {}
    for path in paths:
        try:
            with open(path) as f:
                data = json.load(f)
        except (ValueError, OSError):
            continue
        _declared(data, os.path.relpath(path, mission_dir), out)
    return out


def declaration_of(declarations, sid):
    for key, d in declarations.items():
        if sid == key or (len(key) >= 8 and sid.startswith(key)):
            return d
    return None
