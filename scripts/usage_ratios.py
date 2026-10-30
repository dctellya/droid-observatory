"""Usage report: the Ratios section and the per-kind figures (owner items 10 and 27 of the M4 boundary, B-177).

Imported by scripts/usage-report.py (same directory); Python 3 standard library only.
"""

import os
import re
import subprocess
import sys
import tomllib

CONTRACT_SKILLS = ("core-worker", "ui-worker")
WORKER_SKILLS = ("core-worker", "ui-worker", "infra-worker", "integration-worker")
BUILD_SKILLS = ("core-worker", "ui-worker", "infra-worker")
ASSERTION_KINDS = ("mechanical", "behavioural", "mixed")
UNCLASSIFIED = "unclassified"

ASSERTION_HEADING = re.compile(r"(?m)^(?=#{3,4} VAL-)")
ASSERTION_ID = re.compile(r"#{3,4} (VAL-[A-Za-z0-9-]+)")
KIND_LINE = re.compile(r"(?m)^Kind: *(\w+)")


def load_config(path):
    with open(path, "rb") as f:
        return tomllib.load(f)


def read_kinds(path):
    """{assertion id: kind} from the `Kind:` line under each `### VAL-...` heading of the contract; {} when the file is missing."""
    try:
        with open(path) as f:
            text = f.read()
    except OSError:
        return {}
    kinds = {}
    for block in ASSERTION_HEADING.split(text):
        ident = ASSERTION_ID.match(block)
        if ident:
            kind = KIND_LINE.search(block)
            kinds[ident.group(1)] = kind.group(1) if kind else None
    return kinds


def kind_of(kinds, assertion):
    kind = kinds.get(assertion)
    return kind if kind in ASSERTION_KINDS else UNCLASSIFIED


def cost_per_assertion(worker, coordination, validation, tooling):
    """The worker cost per assertion carries the other three buckets in proportion to the worker's own cost."""
    return worker * (1 + coordination + validation + tooling)


def kind_units(counts, weights):
    """Landed assertions as units of the baseline kind: each kind's count times its weight in `[kind_weights]`."""
    return sum(weights[kind] * n for kind, n in counts.items())


def per(total, count):
    return total / count if count else None


def two(x):
    return "-" if x is None else "%.2f" % x


def claims_of(features, fulfilled):
    """{feature id: fulfilled assertions it claims in `fulfills`}."""
    done = set(fulfilled)
    return {x["id"]: [a for a in dict.fromkeys(x.get("fulfills") or ()) if a in done] for x in features if "id" in x}


def recorded_rows(config):
    rows = []
    for r in config.get("recorded", ()):
        full = all(k in r for k in ("worker_per_assertion", "coordination", "validation", "tooling"))
        rows.append([r["milestone"] + " (recorded)", r.get("assertions", "-"), two(r.get("worker_per_assertion")),
                     two(r.get("coordination")), two(r.get("validation")), two(r.get("tooling")), two(r.get("integration")),
                     two(cost_per_assertion(r["worker_per_assertion"], r["coordination"], r["validation"], r["tooling"])) if full else "-",
                     two(r.get("all_in")), r["source"]])
    return rows


def ratio_lines(emit, md_table, usd, milestone, config, roles, window_usd, feature_rows, features, budgets, fulfilled, kinds,
                window=("-", "-")):
    """Writes Ratios and its five companion tables.

    `roles` is [(role, turns, usd)] of Tokens per role in order; `feature_rows` is [(feature id, skill, turns, usd)] of the window
    (usd includes the subagents a feature's sessions started); `budgets` is {feature id: turnBudget}; `fulfilled` the assertion ids
    passed at the milestone; `kinds` {assertion id: kind}; `window` the (start, end) the report reads.

    The buckets follow the owner's delegate (inbox 0145): the contract worker is the worker cost on features that claim at least one
    assertion in `fulfills`, tooling the worker cost on features that claim none, integration the integration-worker features.
    """
    standard = config["standard_assertion_turns"]
    claims = claims_of(features, fulfilled)
    claimers = {x["id"] for x in features if "id" in x and x.get("fulfills")}
    landed = len(fulfilled)
    weights = config["kind_weights"]
    kind_counts = {k: 0 for k in ASSERTION_KINDS + (UNCLASSIFIED,)}
    for a in fulfilled:
        kind_counts[kind_of(kinds, a)] += 1
    skill_usd = lambda skills: sum(u for _, s, _, u in feature_rows if s in skills)  # noqa: E731
    role_usd = {r: u for r, _, u in roles}
    build_usd = lambda claiming: sum(u for f, s, _, u in feature_rows if s in BUILD_SKILLS and (f in claimers) == claiming)  # noqa: E731
    contract = build_usd(True)
    buckets = [("contract worker", "worker features that claim at least one contract assertion", contract),
               ("coordination", "orchestrator and orchestrator subagent roles", role_usd.get("orchestrator", 0) + role_usd.get("orchestrator subagent", 0)),
               ("validation", "validator role, seats included", role_usd.get("validator", 0)),
               ("tooling", "worker features that claim no contract assertion", build_usd(False)),
               ("integration", "integration-worker features", skill_usd(("integration-worker",)))]
    buckets.append(("self-check and other", "self-check and other roles, worker sessions of features with no skill",
                    window_usd - sum(u for _, _, u in buckets)))
    ratio = {name: per(u, contract) for name, _, u in buckets}
    ratios = [ratio["coordination"], ratio["validation"], ratio["tooling"]]
    bucket_usd = {n: u for n, _, u in buckets}
    worker_pa = per(contract, landed)
    cost_pa = cost_per_assertion(worker_pa, *ratios) if worker_pa is not None and None not in ratios else None
    all_in = per(window_usd, landed)

    emit("### Ratios")
    emit("")
    this = [milestone + " (this run)", landed, two(worker_pa), two(ratio["coordination"]), two(ratio["validation"]),
            two(ratio["tooling"]), two(ratio["integration"]), two(cost_pa), two(all_in),
            "this report: Tokens per role, Feature size, validation-state.json"]
    for r in md_table(["Milestone", "Assertions fulfilled", "Worker cost per assertion", "Coordination ratio", "Validation ratio",
                       "Tooling ratio", "Integration ratio", "Cost per assertion", "All-in cost per assertion", "Source"], recorded_rows(config) + [this]):
        emit(r)
    emit("")
    if cost_pa is None:
        emit("ratios: not available (no contract worker cost or no assertion fulfilled in %s)" % milestone)
    else:
        emit("ratios: worker %s USD-eq per assertion x (1 + %s + %s + %s) = %s USD-eq per assertion (all-in %s)"
             % (two(worker_pa), two(ratios[0]), two(ratios[1]), two(ratios[2]), two(cost_pa), two(all_in)))
    emit("")
    emit("bases: window %s to %s; w = worker cost on features that claim at least one contract assertion %s USD-eq / %d assertions "
         "landed (blocked is not landed); coordination = orchestrator with its subagents %s / w; validation = validators with their "
         "seats %s / w; tooling = worker cost on features that claim none %s / w; integration = %s / w"
         % (window[0], window[1], usd(contract), landed, usd(bucket_usd["coordination"]),
            usd(bucket_usd["validation"]), usd(bucket_usd["tooling"]), usd(bucket_usd["integration"])))
    for r in config.get("recorded", ()):
        if r.get("kinds"):
            emit("")
            emit("%s kinds (recorded): %s" % (r["milestone"], ", ".join("%d %s" % (n, k) for k, n in r["kinds"].items())))
    emit("")
    emit("Cost per assertion = worker cost per assertion x (1 + coordination ratio + validation ratio + tooling ratio) (owner item 10 of "
         "the M4 boundary), on the definition the owner's delegate pinned (inbox 0145). The window is the milestone's build window, the "
         "first own worker dispatch to the integration landing. The contract worker is the worker cost on the features that claim at "
         "least one contract assertion in `fulfills` (their sessions and the subagents they started, fix rounds included), whatever "
         "their skill; w is that cost over the assertions landed (`passed` at the milestone; a blocked assertion is not landed). "
         "Coordination is the orchestrator with its subagents, validation the validators with their seats (scrutiny and user testing), "
         "tooling the worker cost on the features that claim no contract assertion; each ratio is that bucket's USD-eq over the "
         "contract worker's. Integration (the integration-worker features) is its own term, in the table and under it but outside "
         "the product. The self-check missions and the other sessions are in the window total (Cost by bucket) but in no ratio, so the "
         "cost per assertion stays at or under the all-in figure (window total over assertions landed). The recorded rows are the "
         "owner's own derivation, read by hand with other bases (the day's log, a share of all worker cost); the same decomposition "
         "has given different figures on different bases, and this report states its own in the `bases` line.")
    emit("")

    emit("### Cost by bucket")
    emit("")
    rows = [[n, members, usd(u), two(ratio[n])] for n, members, u in buckets]
    rows.append(["window total", "all roles", usd(window_usd), two(per(window_usd, contract))])
    for r in md_table(["Bucket", "Members", "USD-eq", "Ratio to contract worker cost"], rows):
        emit(r)
    emit("")

    emit("### Weighted cost per assertion")
    emit("")
    contract_rows = [(f, u) for f, s, _, u in feature_rows if s in BUILD_SKILLS and f in claimers]
    planned = [(f, u, budgets[f]) for f, u in contract_rows if budgets.get(f) is not None]
    units = sum(b for _, _, b in planned) / standard
    weighted_pa = per(sum(u for _, u, _ in planned), units)

    def weighted(worker):
        return None if worker is None or None in ratios else cost_per_assertion(worker, *ratios)

    kind_total = kind_units(kind_counts, weights)
    kind_pa = per(contract, kind_total)
    for r in md_table(["Basis", "Units", "Worker cost per unit", "Cost per unit"],
                      [["raw assertion", landed, two(worker_pa), two(cost_pa)],
                       ["plan-weighted standard assertion", "%.2f" % units, two(weighted_pa), two(weighted(weighted_pa))],
                       ["kind-weighted mechanical assertion", "%.2f" % kind_total, two(kind_pa), two(weighted(kind_pa))]]):
        emit(r)
    emit("")
    unplanned = len(contract_rows) - len(planned)
    emit("kinds landed: %s; weights %s (unclassified %s) from `config/usage-ratios.toml`; blocked is not landed"
         % (", ".join("%d %s" % (kind_counts[k], k) for k in ASSERTION_KINDS + (UNCLASSIFIED,)),
            " / ".join("%.2f" % weights[k] for k in ASSERTION_KINDS), "%.2f" % weights[UNCLASSIFIED]))
    emit("")
    emit("plan-weighted: %d contract features carry a turnBudget (%d planned turns), %d have none and are left out"
         % (len(planned), sum(b for _, _, b in planned), unplanned))
    emit("")
    emit("Raw is the contract worker cost over the assertions fulfilled; plan-weighted divides the cost of the contract features that "
         "carry a `turnBudget` by their planned turns over %s, so a standard assertion is %s planned turns (the worker turns one "
         "assertion cost on M3's record, `%s`); kind-weighted divides the contract worker cost by the landed assertions counted at their "
         "kind's weight (a mechanical assertion is one unit, so the unit is the cheapest assertion); all three are multiplied by the "
         "same three ratios." % (standard, standard, "config/usage-ratios.toml"))
    emit("")

    worker_rows = [(f, s, t, u) for f, s, t, u in feature_rows if s in WORKER_SKILLS]
    claimed_by_any = {a for f, _, _, _ in worker_rows for a in claims.get(f, ())}
    shares = {k: [0, 0.0, 0] for k in ASSERTION_KINDS + (UNCLASSIFIED,)}
    for k, n in kind_counts.items():
        shares[k][2] = n
    for f, _, turns, cost in worker_rows:
        mine = claims.get(f, ())
        for a in mine:
            shares[kind_of(kinds, a)][0] += turns / len(mine)
            shares[kind_of(kinds, a)][1] += cost / len(mine)
    emit("### Turns per assertion by assertion kind")
    emit("")
    for r in md_table(["Kind", "Assertions", "Attributed worker turns", "Turns per assertion", "Attributed worker USD-eq", "USD-eq per assertion"],
                      [[k, n, "%.1f" % t, two(per(t, n)), usd(c), two(per(c, n))] for k, (t, c, n) in shares.items()]):
        emit(r)
    emit("")
    emit("fulfilled assertions claimed by no worker feature: %d" % len(set(fulfilled) - claimed_by_any))
    emit("")
    emit("The kind of an assertion is the `Kind:` line under its heading in the validation contract (`%s` when it has none). A worker "
         "feature's turns and cost are split equally over the fulfilled assertions it claims in `fulfills`, so an assertion claimed by two "
         "features takes a share of each; features that claim nothing (fix rounds, tooling) are in no kind, and the claims of the "
         "milestone are partial where a feature leaves `fulfills` empty." % UNCLASSIFIED)
    emit("")

    emit("### Turns per assertion by worker skill")
    emit("")

    def skill_row(label, skills):
        rows = [(f, t, u) for f, s, t, u in worker_rows if s in skills]
        turns = sum(t for _, t, _ in rows)
        claimed = {a for f, _, _ in rows for a in claims.get(f, ())}
        return [label, len(rows), turns, usd(sum(u for _, _, u in rows)), len(claimed), two(per(turns, len(claimed))),
                two(per(turns, landed))]

    table = [skill_row(s, (s,)) for s in WORKER_SKILLS] + [skill_row("contract (core + ui)", CONTRACT_SKILLS),
                                                           skill_row("all workers", WORKER_SKILLS)]
    for r in md_table(["Skill", "Features", "Turns", "USD-eq", "Assertions claimed", "Turns per claimed assertion",
                       "Turns per landed assertion"], table):
        emit(r)
    emit("")
    emit("Turns are the log turns of the skill's root sessions in the window (features with turns only); a landed assertion is one "
         "fulfilled in the window, so the per-landed column adds up across skills and the contract row is the D-045 figure "
         "(core and ui turns over all assertions fulfilled); the per-claimed column divides by the fulfilled assertions the skill's "
         "features claim, an upper bound where `fulfills` is partial.")
    emit("")

    emit("### Turns per landed assertion by role")
    emit("")
    rows = [[r, t, landed, two(per(t, landed))] for r, t, _ in roles]
    rows.append(["total", sum(t for _, t, _ in roles), landed, two(per(sum(t for _, t, _ in roles), landed))])
    for r in md_table(["Role", "Turns", "Landed assertions", "Turns per landed assertion"], rows):
        emit(r)
    emit("")


if __name__ == "__main__":
    if sys.argv[1:] == ["self-test"]:
        test = os.path.join(os.path.dirname(os.path.abspath(__file__)), "tests", "usage_ratios_test.py")
        sys.exit(subprocess.run([sys.executable, test], env=dict(os.environ, USAGE_RATIOS_SELF_TEST_DEPTH="1")).returncode)
    sys.exit("usage: usage_ratios.py self-test (runs scripts/tests/usage_ratios_test.py); the module is otherwise imported by usage-report.py")
