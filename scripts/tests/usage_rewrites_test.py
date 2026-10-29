#!/usr/bin/env python3
"""Tests for the lead's context rewrites (owner item 19c) and the weekly allowance share line (owner item 7) of
scripts/usage-report.py, on a scratch copy of the usage fixture (usage_scratch.py) on 2026-10-26.

Fable prices: 10 / 50 / 0.25 / 12.5 per million for input / output / cache read / cache write.
"""

import unittest

from usage_scratch import FABLE, MISSION, SONNET, Scratch, at, table

WORKER = "ffffffff-1111-4111-8111-111111111111"


def lead_scenario(s):
    """Six lead turns around a context of 100 000 tokens (the median of the nonzero cache reads)."""
    s.turn(at("00:10:00"), MISSION, FABLE, cr=100_000, cw=1_000)
    s.turn(at("00:20:00"), MISSION, FABLE, cr=100_000, cw=2_000)
    s.turn(at("00:30:00"), MISSION, FABLE, cr=100_000, cw=250_000)
    s.turn(at("00:40:00"), MISSION, FABLE, cr=100_000, cw=199_999)
    s.turn(at("00:50:00"), MISSION, FABLE, cr=0, cw=100_000)
    s.turn(at("01:00:00"), MISSION, FABLE, cr=100_000, cw=200_400)


class ContextRewrites(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.s = Scratch()
        lead_scenario(cls.s)
        cls.s.worker_sidecar(WORKER, SONNET)
        cls.s.progress("2026-10-26T00:05:00.000Z", "worker_started", workerSessionId=WORKER, featureId="m9-alpha", modelId=SONNET)
        cls.s.turn(at("00:15:00"), WORKER, SONNET, cr=10_000, cw=5_000_000)
        cls.text = cls.s.run().text

    @classmethod
    def tearDownClass(cls):
        cls.s.close()

    def test_a_rewrite_is_a_lead_turn_whose_cache_write_exceeds_its_cache_read_by_the_context_size(self):
        header, rows = table(self.text, "Lead context rewrites")
        self.assertEqual(header, ["Time", "Cache write", "Cache read", "Cache write minus read", "USD-eq"])
        self.assertEqual([(r["Time"], r["Cache write"], r["Cache read"], r["Cache write minus read"], r["USD-eq"]) for r in rows],
                         [(at("00:30:00"), "250000", "100000", "150000", "3.15"),
                          (at("00:50:00"), "100000", "0", "100000", "1.25"),
                          (at("01:00:00"), "200400", "100000", "100400", "2.53")])

    def test_the_context_size_is_the_median_nonzero_cache_read_of_the_lead_in_the_window(self):
        self.assertIn("context size: 100000 tokens (median nonzero cache read of 5 lead turns)", self.text)

    def test_the_summary_gives_turns_tokens_cost_and_share_of_the_lead_cost_per_milestone(self):
        header, rows = table(self.text, "Lead context rewrites per milestone")
        self.assertEqual(header, ["Milestone", "Lead turns", "Rewrite turns", "Context size", "Cache write", "Cache read", "USD-eq",
                                  "Lead USD-eq", "Share of lead cost %"])
        self.assertEqual(rows[0]["Milestone"], "m9")
        self.assertEqual([rows[0][k] for k in header[1:]], ["6", "3", "100000", "550400", "200000", "6.93", "9.54", "72.6"])

    def test_the_summary_line_is_one_line_the_owner_can_compare_per_milestone(self):
        self.assertIn("context rewrites: 3 of 6 lead turns, 550400 cache-write tokens, 6.93 USD-eq (72.6 % of the lead's 9.54)", self.text)

    def test_a_worker_turn_is_not_a_lead_rewrite(self):
        _, rows = table(self.text, "Lead context rewrites")
        self.assertNotIn(at("00:15:00"), [r["Time"] for r in rows])

    def test_the_caption_states_the_rule_and_that_the_context_is_measured_not_fixed(self):
        self.assertIn("A rewrite is a lead turn whose cache-write tokens exceed its cache-read tokens by at least the context size", self.text)

    def test_a_lead_without_cache_reads_has_no_context_and_no_rewrites(self):
        s = Scratch()
        try:
            s.turn(at("00:10:00"), MISSION, FABLE, i=1_000, cw=9_000_000)
            text = s.run().text
        finally:
            s.close()
        self.assertIn("context rewrites: none (no lead turn with a cache read in the window)", text)
        self.assertEqual(table(text, "Lead context rewrites")[1][0]["Time"], "none")


LINE = "orchestrator turns per feature boundary: "


def completed(s, ts, feature="m9-x", kind="worker_completed"):
    s.progress(ts, kind, workerSessionId="ffffffff-2222-4222-8222-222222222222", featureId=feature)


def flat_turns(s, cost_usd_each=(1, 1, 1, 1, 1, 1)):
    """Six lead turns without cache reads (so no context and no rewrite); 20 000 output tokens cost 1.00 USD-eq."""
    for hhmm, c in zip(("00:10:00", "00:20:00", "00:30:00", "00:40:00", "00:50:00", "01:00:00"), cost_usd_each):
        s.turn(at(hhmm), MISSION, FABLE, o=20_000 * c)


class BoundaryTurns(unittest.TestCase):
    def run_with(self, scenario):
        s = Scratch()
        try:
            scenario(s)
            return s.run().text
        finally:
            s.close()

    def test_the_line_divides_the_lead_turns_by_the_worker_hand_offs_of_the_window(self):
        def scenario(s):
            flat_turns(s)
            completed(s, at("00:15:00"))
            completed(s, at("00:45:00"))

        text = self.run_with(scenario)
        self.assertIn(LINE + "6 / 2 = 3.0 (target 3)", text)

    def test_the_line_sits_in_the_lead_context_rewrites_section(self):
        def scenario(s):
            flat_turns(s)
            completed(s, at("00:15:00"))

        text = self.run_with(scenario)
        section = text.split("### Lead context rewrites per milestone")[1].split("\n### ")[0]
        self.assertIn(LINE + "6 / 1 = 6.0 (target 3)", section)

    def test_without_a_compaction_marker_the_one_line_stands_alone(self):
        def scenario(s):
            flat_turns(s)
            completed(s, at("00:15:00"))
            completed(s, at("00:45:00"))

        self.assertNotIn("before compaction:", self.run_with(scenario))

    def test_only_completed_hand_offs_inside_the_window_count(self):
        def scenario(s):
            flat_turns(s)
            completed(s, at("00:15:00"))
            completed(s, at("00:25:00"), kind="worker_failed")
            completed(s, "2026-10-25T23:00:00.000Z")
            completed(s, "2026-10-26T07:00:00.000Z")

        self.assertIn(LINE + "6 / 1 = 6.0 (target 3)", self.run_with(scenario))

    def test_without_a_hand_off_the_ratio_is_not_available(self):
        def scenario(s):
            flat_turns(s)

        text = self.run_with(scenario)
        self.assertIn(LINE + "6 / 0 = n/a (target 3)", text)

    def test_a_mission_resumed_line_splits_the_ratio_and_the_cost_per_turn(self):
        def scenario(s):
            flat_turns(s, (2, 2, 2, 1, 1, 1))
            completed(s, at("00:05:00"))
            completed(s, at("00:15:00"))
            s.progress(at("00:35:00"), "mission_resumed")
            completed(s, at("00:45:00"))
            completed(s, at("00:55:00"))
            completed(s, at("01:05:00"))

        text = self.run_with(scenario)
        self.assertIn(LINE + "6 / 5 = 1.2 (target 3)", text)
        self.assertIn("before compaction: 3 turns / 2 boundaries = 1.5, 2.00 USD-eq per turn; "
                      "after: 3 turns / 3 boundaries = 1.0, 1.00 USD-eq per turn "
                      "(marker: mission_resumed %s)" % at("00:35:00"), text)

    def test_a_lead_context_rewrite_is_a_marker_too(self):
        def scenario(s):
            lead_scenario(s)
            completed(s, at("00:15:00"))
            completed(s, at("00:35:00"))
            completed(s, at("00:45:00"))

        text = self.run_with(scenario)
        self.assertIn("before compaction: 2 turns / 1 boundaries = 2.0, 0.04 USD-eq per turn; "
                      "after: 4 turns / 2 boundaries = 2.0, 2.36 USD-eq per turn "
                      "(marker: lead context rewrite %s)" % at("00:30:00"), text)

    def test_the_earliest_marker_splits(self):
        def scenario(s):
            lead_scenario(s)
            completed(s, at("00:15:00"))
            s.progress(at("00:25:00"), "mission_resumed")

        text = self.run_with(scenario)
        self.assertIn("before compaction: 2 turns / 1 boundaries = 2.0", text)
        self.assertIn("(marker: mission_resumed %s)" % at("00:25:00"), text)

    def test_a_side_without_a_hand_off_prints_not_available(self):
        def scenario(s):
            flat_turns(s)
            completed(s, at("00:45:00"))
            s.progress(at("00:35:00"), "mission_resumed")

        text = self.run_with(scenario)
        self.assertIn("before compaction: 3 turns / 0 boundaries = n/a, 1.00 USD-eq per turn; "
                      "after: 3 turns / 1 boundaries = 3.0, 1.00 USD-eq per turn", text)


def allowance_scenario(s, next_features):
    s.turn(at("00:10:00"), MISSION, FABLE, i=1_000_000)
    s.features(next_features)


NEXT = [{"id": "m10-a", "milestone": "m10", "status": "pending", "fulfills": ["VAL-B-001", "VAL-B-002", "VAL-B-003"]},
        {"id": "m10-b", "milestone": "m10", "status": "pending", "fulfills": ["VAL-B-003", "VAL-B-004"]},
        {"id": "m11-a", "milestone": "m11", "status": "pending", "fulfills": ["VAL-C-001"]}]


class WeeklyAllowance(unittest.TestCase):
    def run_with(self, extra=(), features=NEXT):
        s = Scratch()
        try:
            allowance_scenario(s, features)
            return s.run(extra).text
        finally:
            s.close()

    def test_the_line_multiplies_the_cost_per_assertion_by_the_next_milestones_claimed_assertions(self):
        self.assertIn("weekly allowance share: m10 claims 4 assertions x 3.33 USD-eq per assertion (m9 window, 3 fulfilled) = 13.33 USD-eq",
                      self.run_with())

    def test_the_next_milestone_can_be_named(self):
        self.assertIn("weekly allowance share: m11 claims 1 assertions x 3.33 USD-eq per assertion (m9 window, 3 fulfilled) = 3.33 USD-eq",
                      self.run_with(["--next-milestone", "m11"]))

    def test_without_a_later_milestone_the_line_says_so(self):
        self.assertIn("weekly allowance share: none (no milestone after m9 in features.json)", self.run_with(features=[]))

    def test_without_a_fulfilled_assertion_the_line_says_it_cannot_price_one(self):
        s = Scratch()
        try:
            allowance_scenario(s, NEXT)
            text = s.run(milestone="m7").text
        finally:
            s.close()
        self.assertIn("weekly allowance share: unavailable (no assertion fulfilled in m7)", text)

    def test_the_caption_says_the_orchestrator_states_it_before_dispatch(self):
        self.assertIn("The orchestrator states the weekly allowance share before it dispatches the next milestone", self.run_with())


if __name__ == "__main__":
    unittest.main()
