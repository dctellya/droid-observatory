#!/usr/bin/env python3
"""Tests for the Ratios section of scripts/usage-report.py (owner item 10 and 27 of the M4 boundary) and for the
`single-call share` line (B-176), on a scratch copy of the usage fixture (usage_scratch.py) on 2026-10-26.

The cost buckets follow the delegate's pinned definition (inbox 0145): the contract worker is the worker cost on features that claim
at least one assertion in `fulfills`, tooling the worker cost on features that claim none, integration its own term.

Prices: fable 10 / 50 / 0.25 / 12.5, sonnet 2 / 10 / 0.1 / 2.5, opus 4 / 20 / 0.2 / 5 per million for input / output / cache
read / cache write, so 20 000 fable output tokens, 100 000 sonnet output tokens and 50 000 opus output tokens each cost 1.00 USD-eq.
"""

import os
import sys
import unittest

from usage_scratch import FABLE, MISSION, OPUS, SONNET, Scratch, at, table

CORE, UI, TOOL, CHECKER = ("aaaaaaaa-1111-4111-8111-111111111111", "aaaaaaaa-2222-4222-8222-222222222222",
                           "aaaaaaaa-3333-4333-8333-333333333333", "aaaaaaaa-4444-4444-8444-444444444444")

CONTRACT = """# Contract

### VAL-A-001: first
Kind: mechanical
text

### VAL-A-002: second
Kind: behavioural
text

#### VAL-A-003: third
Kind: behavioural
text
"""

FEATURES = [
    {"id": "m9-core", "milestone": "m9", "skillName": "core-worker", "status": "completed", "turnBudget": 69.9,
     "fulfills": ["VAL-A-001", "VAL-A-002"]},
    {"id": "m9-ui", "milestone": "m9", "skillName": "ui-worker", "status": "completed", "turnBudget": 23.3, "fulfills": ["VAL-A-003"]},
    {"id": "m9-tool", "milestone": "m9", "skillName": "infra-worker", "status": "completed"},
]


def start(s, sid, feature, ts, model=SONNET):
    s.worker_sidecar(sid, model)
    s.progress(ts, "worker_started", workerSessionId=sid, featureId=feature, modelId=model)


def scenario(s, contract=CONTRACT):
    """Six lead turns (6.00), three core turns (3.00), two ui turns (2.00), one infra turn (1.00), one validator turn (1.00)."""
    for hhmm in ("00:10:00", "00:20:00", "00:30:00", "00:40:00", "00:50:00", "01:00:00"):
        s.turn(at(hhmm), MISSION, FABLE, o=20_000)
    s.features(FEATURES)
    start(s, CORE, "m9-core", "2026-10-26T00:01:00.000Z")
    start(s, UI, "m9-ui", "2026-10-26T00:02:00.000Z")
    start(s, TOOL, "m9-tool", "2026-10-26T00:03:00.000Z")
    start(s, CHECKER, "scrutiny-validator-m9", "2026-10-26T00:04:00.000Z", OPUS)
    for hhmm in ("00:11:00", "00:12:00", "00:13:00"):
        s.turn(at(hhmm), CORE, SONNET, o=100_000)
    for hhmm in ("00:21:00", "00:22:00"):
        s.turn(at(hhmm), UI, SONNET, o=100_000)
    s.turn(at("00:31:00"), TOOL, SONNET, o=100_000)
    s.turn(at("00:41:00"), CHECKER, OPUS, o=50_000)
    if contract is not None:
        with open(os.path.join(s.mission, "validation-contract.md"), "w") as f:
            f.write(contract)


class RatiosSection(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.s = Scratch()
        scenario(cls.s)
        cls.text = cls.s.run().text

    @classmethod
    def tearDownClass(cls):
        cls.s.close()

    def rows(self, heading, key):
        return {r[key]: r for r in table(self.text, heading)[1]}

    def test_the_cost_per_assertion_is_the_worker_cost_per_assertion_times_one_plus_the_three_ratios(self):
        header, rows = table(self.text, "Ratios")
        self.assertEqual(header, ["Milestone", "Assertions fulfilled", "Worker cost per assertion", "Coordination ratio",
                                  "Validation ratio", "Tooling ratio", "Integration ratio", "Cost per assertion",
                                  "All-in cost per assertion", "Source"])
        mine = next(r for r in rows if r["Milestone"] == "m9 (this run)")
        self.assertEqual([mine[k] for k in header[1:9]], ["3", "1.67", "1.20", "0.20", "0.20", "0.00", "4.33", "4.33"])

    def test_the_summary_line_shows_the_product(self):
        self.assertIn("ratios: worker 1.67 USD-eq per assertion x (1 + 1.20 + 0.20 + 0.20) = 4.33 USD-eq per assertion (all-in 4.33)",
                      self.text)

    def test_the_owners_m4_figures_are_the_first_data_row_and_m3_comes_before_them(self):
        _, rows = table(self.text, "Ratios")
        self.assertEqual([r["Milestone"] for r in rows], ["m3 (recorded)", "m4 (recorded)", "m9 (this run)"])
        m4 = rows[1]
        self.assertEqual([m4[k] for k in ("Assertions fulfilled", "Worker cost per assertion", "Coordination ratio", "Validation ratio",
                                          "Tooling ratio", "Cost per assertion", "All-in cost per assertion")],
                         ["33", "1.06", "2.86", "1.30", "0.85", "6.37", "6.70"])
        self.assertIn("owner item 10", m4["Source"])

    def test_the_m3_row_carries_only_the_two_ratios_the_owner_read(self):
        m3 = table(self.text, "Ratios")[1][0]
        self.assertEqual([m3[k] for k in ("Worker cost per assertion", "Coordination ratio", "Validation ratio", "Tooling ratio",
                                          "Cost per assertion")], ["-", "0.90", "1.40", "-", "-"])

    def test_the_buckets_reconcile_to_the_window_total(self):
        header, rows = table(self.text, "Cost by bucket")
        self.assertEqual(header, ["Bucket", "Members", "USD-eq", "Ratio to contract worker cost"])
        got = {r["Bucket"]: (r["USD-eq"], r["Ratio to contract worker cost"]) for r in rows}
        self.assertEqual(got, {"contract worker": ("5.00", "1.00"), "coordination": ("6.00", "1.20"), "validation": ("1.00", "0.20"),
                               "tooling": ("1.00", "0.20"), "integration": ("0.00", "0.00"), "self-check and other": ("0.00", "0.00"),
                               "window total": ("13.00", "2.60")})

    def test_the_raw_cost_per_assertion_sits_next_to_the_one_weighted_by_the_plan_turn_budget(self):
        header, rows = table(self.text, "Weighted cost per assertion")
        self.assertEqual(header, ["Basis", "Units", "Worker cost per unit", "Cost per unit"])
        got = {r["Basis"]: (r["Units"], r["Worker cost per unit"], r["Cost per unit"]) for r in rows}
        self.assertEqual(got["raw assertion"], ("3", "1.67", "4.33"))
        self.assertEqual(got["plan-weighted standard assertion"], ("4.00", "1.25", "3.25"))
        self.assertIn("a standard assertion is 23.3 planned turns", self.text)

    def test_the_kind_weighted_figure_sits_next_to_the_raw_and_the_plan_weighted_ones(self):
        header, rows = table(self.text, "Weighted cost per assertion")
        self.assertEqual([r["Basis"] for r in rows], ["raw assertion", "plan-weighted standard assertion",
                                                      "kind-weighted mechanical assertion"])
        got = {r["Basis"]: (r["Units"], r["Worker cost per unit"], r["Cost per unit"]) for r in rows}
        self.assertEqual(got["kind-weighted mechanical assertion"], ("3.50", "1.43", "3.71"))
        self.assertIn("kinds landed: 1 mechanical, 2 behavioural, 0 mixed, 0 unclassified; weights 1.00 / 1.25 / 1.50 "
                      "(unclassified 1.00) from `config/usage-ratios.toml`", self.text)

    def test_the_four_bases_are_printed_under_the_ratios_table(self):
        section = self.text.split("### Ratios")[1].split("\n### ")[0]
        line = next(x for x in section.splitlines() if x.startswith("bases: "))
        self.assertIn("window 2026-10-26T00:00:00Z to 2026-10-26T06:00:00Z", line)
        self.assertIn("w = worker cost on features that claim at least one contract assertion 5.00 USD-eq / 3 assertions landed", line)
        self.assertIn("coordination = orchestrator with its subagents 6.00 / w", line)
        self.assertIn("validation = validators with their seats 1.00 / w", line)
        self.assertIn("tooling = worker cost on features that claim none 1.00 / w", line)
        self.assertIn("integration = 0.00 / w", line)
        self.assertLess(section.index("| m9 (this run)"), section.index("bases: "))

    def test_the_recorded_m4_kinds_are_the_products_count_blocked_is_not_landed(self):
        self.assertIn("m4 kinds (recorded): 12 mechanical, 14 behavioural, 6 mixed", self.text)

    def test_per_kind_figures_split_a_feature_over_the_assertions_it_claims(self):
        header, rows = table(self.text, "Turns per assertion by assertion kind")
        self.assertEqual(header, ["Kind", "Assertions", "Attributed worker turns", "Turns per assertion", "Attributed worker USD-eq",
                                  "USD-eq per assertion"])
        got = {r["Kind"]: [r[k] for k in header[1:]] for r in rows}
        self.assertEqual(got["mechanical"], ["1", "1.5", "1.50", "1.50", "1.50"])
        self.assertEqual(got["behavioural"], ["2", "3.5", "1.75", "3.50", "1.75"])
        self.assertEqual(got["mixed"], ["0", "0.0", "-", "0.00", "-"])
        self.assertEqual(got["unclassified"], ["0", "0.0", "-", "0.00", "-"])
        self.assertIn("fulfilled assertions claimed by no worker feature: 0", self.text)

    def test_per_skill_turns_per_assertion_are_given_for_core_ui_infra_and_integration(self):
        header, rows = table(self.text, "Turns per assertion by worker skill")
        self.assertEqual(header, ["Skill", "Features", "Turns", "USD-eq", "Assertions claimed", "Turns per claimed assertion",
                                  "Turns per landed assertion"])
        got = {r["Skill"]: [r[k] for k in header[1:]] for r in rows}
        self.assertEqual(got["core-worker"], ["1", "3", "3.00", "2", "1.50", "1.00"])
        self.assertEqual(got["ui-worker"], ["1", "2", "2.00", "1", "2.00", "0.67"])
        self.assertEqual(got["infra-worker"], ["1", "1", "1.00", "0", "-", "0.33"])
        self.assertEqual(got["integration-worker"], ["0", "0", "0.00", "0", "-", "0.00"])
        self.assertEqual(got["contract (core + ui)"], ["2", "5", "5.00", "3", "1.67", "1.67"])
        self.assertEqual(got["all workers"], ["3", "6", "6.00", "3", "2.00", "2.00"])

    def test_turns_per_landed_assertion_are_given_by_role(self):
        header, rows = table(self.text, "Turns per landed assertion by role")
        self.assertEqual(header, ["Role", "Turns", "Landed assertions", "Turns per landed assertion"])
        got = {r["Role"]: [r[k] for k in header[1:]] for r in rows}
        self.assertEqual(got["orchestrator"], ["6", "3", "2.00"])
        self.assertEqual(got["worker"], ["6", "3", "2.00"])
        self.assertEqual(got["validator"], ["1", "3", "0.33"])
        self.assertEqual(got["orchestrator subagent"], ["0", "3", "0.00"])
        self.assertEqual(got["total"], ["13", "3", "4.33"])

    def test_the_caption_states_the_buckets_and_that_the_recorded_rows_are_the_owners_hand_derivation(self):
        self.assertIn("contract worker is the worker cost on the features that claim at least one contract assertion", self.text)
        self.assertIn("inbox 0145", self.text)
        self.assertIn("recorded rows are the owner's own derivation", self.text)


class RatiosWithoutData(unittest.TestCase):
    def run_with(self, **kw):
        s = Scratch()
        try:
            scenario(s, **kw)
            return s.run().text
        finally:
            s.close()

    def test_without_a_contract_file_every_fulfilled_assertion_is_unclassified(self):
        text = self.run_with(contract=None)
        got = {r["Kind"]: r["Assertions"] for r in table(text, "Turns per assertion by assertion kind")[1]}
        self.assertEqual(got, {"mechanical": "0", "behavioural": "0", "mixed": "0", "unclassified": "3"})

    def test_a_milestone_without_a_fulfilled_assertion_shows_dashes_not_zeros_for_the_per_assertion_figures(self):
        s = Scratch()
        try:
            scenario(s)
            text = s.run(milestone="m7").text
        finally:
            s.close()
        mine = next(r for r in table(text, "Ratios")[1] if r["Milestone"] == "m7 (this run)")
        self.assertEqual((mine["Assertions fulfilled"], mine["Worker cost per assertion"], mine["Cost per assertion"],
                          mine["All-in cost per assertion"]), ("0", "-", "-", "-"))

    def test_without_a_contract_feature_cost_the_ratios_are_not_available(self):
        s = Scratch()
        try:
            for hhmm in ("00:10:00", "00:20:00"):
                s.turn(at(hhmm), MISSION, FABLE, o=20_000)
            text = s.run().text
        finally:
            s.close()
        mine = next(r for r in table(text, "Ratios")[1] if r["Milestone"] == "m9 (this run)")
        self.assertEqual((mine["Worker cost per assertion"], mine["Coordination ratio"], mine["Cost per assertion"]), ("0.00", "-", "-"))


class PinnedDefinition(unittest.TestCase):
    """Delegate's definition (inbox 0145): the buckets follow `fulfills`, not the skill; integration is its own term."""

    @classmethod
    def setUpClass(cls):
        cls.s = Scratch()
        s = cls.s
        for hhmm in ("00:10:00", "00:20:00", "00:30:00", "00:40:00", "00:50:00", "01:00:00"):
            s.turn(at(hhmm), MISSION, FABLE, o=20_000)
        s.features([
            {"id": "m9-core", "milestone": "m9", "skillName": "core-worker", "status": "completed", "fulfills": ["VAL-A-001", "VAL-A-002"]},
            {"id": "m9-fix", "milestone": "m9", "skillName": "core-worker", "status": "completed", "fulfills": []},
            {"id": "m9-infra", "milestone": "m9", "skillName": "infra-worker", "status": "completed", "fulfills": ["VAL-A-003"]},
            {"id": "m9-integ", "milestone": "m9", "skillName": "integration-worker", "status": "completed", "fulfills": []},
        ])
        sessions = {"m9-core": ("bbbbbbbb-1111-4111-8111-111111111111", 3), "m9-fix": ("bbbbbbbb-2222-4222-8222-222222222222", 1),
                    "m9-infra": ("bbbbbbbb-3333-4333-8333-333333333333", 2), "m9-integ": ("bbbbbbbb-4444-4444-8444-444444444444", 1)}
        for i, (feature, (sid, turns)) in enumerate(sessions.items()):
            start(s, sid, feature, "2026-10-26T00:0%d:00.000Z" % (i + 1))
            for n in range(turns):
                s.turn(at("00:%d%d:00" % (i + 1, n + 1)), sid, SONNET, o=100_000)
        start(s, CHECKER, "scrutiny-validator-m9", "2026-10-26T00:05:00.000Z", OPUS)
        s.turn(at("00:41:00"), CHECKER, OPUS, o=50_000)
        s.subagent_sidecar("cccccccc-1111-4111-8111-111111111111", CHECKER, "toolu_seat", OPUS)
        s.turn(at("00:42:00"), "cccccccc-1111-4111-8111-111111111111", OPUS, o=50_000)
        with open(os.path.join(s.mission, "validation-contract.md"), "w") as f:
            f.write(CONTRACT)
        cls.text = s.run().text

    @classmethod
    def tearDownClass(cls):
        cls.s.close()

    def bucket(self, name):
        return {r["Bucket"]: r for r in table(self.text, "Cost by bucket")[1]}[name]

    def test_the_contract_worker_is_the_worker_cost_on_features_that_claim_an_assertion_whatever_their_skill(self):
        self.assertEqual(self.bucket("contract worker")["USD-eq"], "5.00")

    def test_a_core_worker_feature_that_claims_none_is_tooling(self):
        self.assertEqual(self.bucket("tooling")["USD-eq"], "1.00")
        self.assertEqual(self.bucket("tooling")["Ratio to contract worker cost"], "0.20")

    def test_integration_is_its_own_term_over_the_contract_worker_cost(self):
        self.assertEqual(self.bucket("integration")["USD-eq"], "1.00")
        mine = next(r for r in table(self.text, "Ratios")[1] if r["Milestone"] == "m9 (this run)")
        self.assertEqual((mine["Integration ratio"], mine["Cost per assertion"]), ("0.20", "4.67"))

    def test_the_validators_seats_are_in_the_validation_ratio(self):
        self.assertEqual(self.bucket("validation")["USD-eq"], "2.00")
        self.assertIn("validation = validators with their seats 2.00 / w", self.text)


class KindWeights(unittest.TestCase):
    """Kind weights of config/usage-ratios.toml (mechanical 1.0, behavioural 1.25, mixed 1.5)."""

    CONFIG = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "config", "usage-ratios.toml")

    def setUp(self):
        sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
        import usage_ratios
        self.ratios = usage_ratios

    def test_the_config_carries_the_three_weights(self):
        weights = self.ratios.load_config(self.CONFIG)["kind_weights"]
        self.assertEqual((weights["mechanical"], weights["behavioural"], weights["mixed"]), (1.0, 1.25, 1.5))

    def test_every_weight_has_a_one_line_reason_above_it(self):
        lines = open(self.CONFIG).read().splitlines()
        start = lines.index("[kind_weights]")
        for i, line in enumerate(lines[start + 1:], start + 1):
            if line.startswith("["):
                break
            if "=" in line and not line.startswith("#"):
                self.assertTrue(lines[i - 1].startswith("# "), "no reason above: " + line)

    def test_the_m4_kinds_twelve_fourteen_six_weigh_thirty_eight_and_a_half_units(self):
        weights = self.ratios.load_config(self.CONFIG)["kind_weights"]
        counts = {"mechanical": 12, "behavioural": 14, "mixed": 6, "unclassified": 0}
        self.assertEqual(self.ratios.kind_units(counts, weights), 38.5)

    def test_an_unclassified_assertion_weighs_the_unclassified_weight(self):
        self.assertEqual(self.ratios.kind_units({"unclassified": 2}, {"unclassified": 1.0}), 2.0)

    def test_the_m4_recorded_row_counts_fourteen_behavioural(self):
        recorded = {r["milestone"]: r for r in self.ratios.load_config(self.CONFIG)["recorded"]}
        self.assertEqual(recorded["m4"]["kinds"], {"mechanical": 12, "behavioural": 14, "mixed": 6})


class SelfTest(unittest.TestCase):
    @unittest.skipIf(os.environ.get("USAGE_RATIOS_SELF_TEST_DEPTH"), "this run is the script's own self-test")
    def test_the_script_runs_its_own_self_test(self):
        import subprocess
        script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "usage_ratios.py")
        proc = subprocess.run([sys.executable, script, "self-test"], capture_output=True, text=True,
                              env=dict(os.environ, USAGE_RATIOS_SELF_TEST_DEPTH="1"))
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn("Ran ", proc.stderr)
        self.assertIn("OK", proc.stderr)


class RatioFormula(unittest.TestCase):
    def test_the_owners_m4_figures_give_six_point_four(self):
        sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
        import usage_ratios
        self.assertEqual(round(usage_ratios.cost_per_assertion(1.06, 2.86, 1.30, 0.85), 1), 6.4)
        self.assertEqual("%.2f" % usage_ratios.cost_per_assertion(1.06, 2.86, 1.30, 0.85), "6.37")


def record(ts, calls, rid):
    content = [{"type": "text", "text": "x"}] + [{"type": "tool_use", "id": "%s-%d" % (rid, i), "name": "Execute",
                                                    "input": {"command": "true"}} for i in range(calls)]
    return {"id": rid, "timestamp": ts, "message": {"role": "assistant", "content": content}}


class SingleCallShare(unittest.TestCase):
    LINE = "single-call share: "

    def run_with(self, records):
        s = Scratch()
        try:
            s.turn(at("00:10:00"), MISSION, FABLE, o=20_000)
            s.progress(at("00:15:00"), "worker_completed", workerSessionId="ffffffff-2222-4222-8222-222222222222",
                       featureId="m9-x")
            s.transcript(MISSION, records)
            return s.run().text
        finally:
            s.close()

    def test_the_line_counts_the_turns_that_make_exactly_one_tool_call(self):
        text = self.run_with([record(at("00:10:00"), 1, "a"), record(at("00:11:00"), 1, "b"), record(at("00:12:00"), 2, "c"),
                              record(at("00:13:00"), 0, "d"), record(at("00:14:00"), 1, "e")])
        self.assertIn(self.LINE + "3 / 4 = 75.0 % (turns with exactly one tool call over turns with at least one; "
                      "1 turn with two or more calls, 1 turn with none)", text)

    def test_the_line_sits_right_under_the_orchestrator_turns_per_feature_boundary_line(self):
        text = self.run_with([record(at("00:10:00"), 1, "a")])
        section = text.split("### Lead context rewrites per milestone")[1].split("\n### ")[0]
        self.assertIn("orchestrator turns per feature boundary: ", section)
        self.assertLess(section.index("orchestrator turns per feature boundary: "), section.index(self.LINE))

    def test_turns_outside_the_window_are_not_counted(self):
        text = self.run_with([record(at("00:10:00"), 1, "a"), record("2026-10-27T00:10:00.000Z", 2, "b"),
                              record("2026-10-25T23:00:00.000Z", 2, "c")])
        self.assertIn(self.LINE + "1 / 1 = 100.0 %", text)

    def test_without_a_transcript_turn_the_share_is_not_available(self):
        s = Scratch()
        try:
            s.turn(at("00:10:00"), MISSION, FABLE, o=20_000)
            text = s.run().text
        finally:
            s.close()
        self.assertIn(self.LINE + "n/a (no orchestrator transcript turn with a tool call in the window)", text)


if __name__ == "__main__":
    unittest.main()
