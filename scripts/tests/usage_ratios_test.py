#!/usr/bin/env python3
"""Tests for the Ratios section of scripts/usage-report.py (owner item 10 and 27 of the M4 boundary) and for the
`single-call share` line (B-176), on a scratch copy of the usage fixture (usage_scratch.py) on 2026-10-26.

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
                                  "Validation ratio", "Tooling ratio", "Cost per assertion", "All-in cost per assertion", "Source"])
        mine = next(r for r in rows if r["Milestone"] == "m9 (this run)")
        self.assertEqual([mine[k] for k in header[1:8]], ["3", "1.67", "1.20", "0.20", "0.20", "4.33", "4.33"])

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
        self.assertIn("contract worker is the cost of the features whose skill is core-worker or ui-worker", self.text)
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
