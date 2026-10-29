#!/usr/bin/env python3
"""Tests for the user testing cost tables, the lead's context rewrites and the weekly allowance line of scripts/usage-report.py
(owner items 7, 12 and 19c of the M3 boundary), on a scratch copy of the usage fixture (usage_scratch.py) on 2026-10-26.

Opus prices: 4 / 20 / 0.2 / 5 per million for input / output / cache read / cache write; fable 10 / 50 / 0.25 / 12.5.
"""

import unittest

from usage_scratch import FABLE, MISSION, OPUS, Scratch, at, table

UT1 = "aaaaaaaa-1111-4111-8111-111111111111"
UT2 = "aaaaaaaa-2222-4222-8222-222222222222"
FG1 = "bbbbbbbb-1111-4111-8111-111111111111"
FG2 = "bbbbbbbb-2222-4222-8222-222222222222"
FG3 = "bbbbbbbb-3333-4333-8333-333333333333"
NESTED = "cccccccc-1111-4111-8111-111111111111"
CORE = "Test M9 core-shell assertions"
BOARD = "Test M9 browser board assertions"
LIVE = "Test M9 live host controls"


def task(ts, call_id, description):
    return {"id": "r" + call_id, "timestamp": ts, "message": {"role": "assistant", "content": [
        {"type": "tool_use", "id": call_id, "name": "Task", "input": {"subagent_type": "user-testing-flow-validator",
                                                                     "description": description, "prompt": "p"}}]}}


def testing_scenario(s):
    s.progress("2026-10-26T02:00:00.000Z", "worker_started", workerSessionId=UT1, featureId="user-testing-validator-m9", modelId=OPUS)
    s.progress("2026-10-26T04:00:00.000Z", "worker_started", workerSessionId=UT2, featureId="user-testing-validator-m9", modelId=OPUS)
    for sid in (UT1, UT2):
        s.worker_sidecar(sid)
    s.subagent_sidecar(FG1, UT1, "tu_a")
    s.subagent_sidecar(FG2, UT1, "tu_b")
    s.subagent_sidecar(NESTED, FG1, "tu_n")
    s.subagent_sidecar(FG3, UT2, "tu_c")
    s.transcript(UT1, [task("2026-10-26T02:02:00.000Z", "tu_a", CORE), task("2026-10-26T02:02:00.000Z", "tu_b", BOARD)])
    s.transcript(UT2, [task("2026-10-26T04:02:00.000Z", "tu_c", LIVE)])
    s.turn(at("02:01:00"), UT1, OPUS, i=1_000_000)
    s.turn(at("02:05:00"), FG1, OPUS, o=100_000)
    s.turn(at("02:06:00"), NESTED, OPUS, i=250_000)
    s.turn(at("02:10:00"), FG2, OPUS, i=500_000, cw=200_000)
    s.turn(at("04:01:00"), UT2, OPUS, o=50_000)
    s.turn(at("04:05:00"), FG3, OPUS, cr=10_000_000)


class UserTesting(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.s = Scratch()
        testing_scenario(cls.s)
        cls.text = cls.s.run().text

    @classmethod
    def tearDownClass(cls):
        cls.s.close()

    def test_a_round_is_a_validator_session_and_costs_it_with_the_subagents_it_started(self):
        header, rows = table(self.text, "User testing rounds")
        self.assertEqual(header, ["Round", "Session", "Flow groups", "Subagent sessions", "Validator turns", "Validator USD-eq",
                                  "Subagent USD-eq", "Total USD-eq", "First ts", "Last ts"])
        got = [(r["Round"], r["Session"], r["Flow groups"], r["Subagent sessions"], r["Validator turns"], r["Validator USD-eq"],
                r["Subagent USD-eq"], r["Total USD-eq"]) for r in rows]
        self.assertEqual(got, [("1", "`%s`" % UT1, "2", "3", "1", "4.00", "6.00", "10.00"),
                               ("2", "`%s`" % UT2, "1", "1", "1", "1.00", "2.00", "3.00")])
        self.assertEqual((rows[0]["First ts"], rows[0]["Last ts"]), (at("02:01:00"), at("02:10:00")))

    def test_a_flow_group_is_a_task_of_the_validator_with_the_cost_of_the_subagents_under_it(self):
        header, rows = table(self.text, "User testing flow groups")
        self.assertEqual(header, ["Round", "Group", "Subagent sessions", "Turns", "USD-eq", "Models", "First ts", "Last ts"])
        got = [(r["Round"], r["Group"], r["Subagent sessions"], r["Turns"], r["USD-eq"], r["Models"]) for r in rows]
        self.assertEqual(got, [("1", CORE, "2", "2", "3.00", OPUS), ("1", BOARD, "1", "1", "3.00", OPUS), ("2", LIVE, "1", "1", "2.00", OPUS)])

    def test_the_total_line_adds_rounds_groups_and_the_two_kinds_of_session(self):
        self.assertIn("user testing: 2 rounds, 3 flow groups, 13.00 USD-eq (validator sessions 5.00, subagents 8.00)", self.text)

    def test_a_subagent_whose_task_is_not_in_the_transcript_is_a_group_of_its_own_named_so(self):
        s = Scratch()
        try:
            testing_scenario(s)
            s.subagent_sidecar("dddddddd-1111-4111-8111-111111111111", UT2, "tu_missing")
            s.turn(at("04:10:00"), "dddddddd-1111-4111-8111-111111111111", OPUS, i=250_000)
            text = s.run().text
        finally:
            s.close()
        _, rows = table(text, "User testing flow groups")
        self.assertIn(("2", "(no Task description found)", "1", "1", "1.00"),
                      [(r["Round"], r["Group"], r["Subagent sessions"], r["Turns"], r["USD-eq"]) for r in rows])

    def test_a_scrutiny_validator_is_not_user_testing(self):
        s = Scratch()
        try:
            testing_scenario(s)
            s.progress("2026-10-26T03:00:00.000Z", "worker_started", workerSessionId="eeeeeeee-1111-4111-8111-111111111111",
                       featureId="scrutiny-validator-m9", modelId=OPUS)
            s.worker_sidecar("eeeeeeee-1111-4111-8111-111111111111")
            s.turn(at("03:01:00"), "eeeeeeee-1111-4111-8111-111111111111", OPUS, i=1_000_000)
            text = s.run().text
        finally:
            s.close()
        self.assertEqual(len(table(text, "User testing rounds")[1]), 2)
        self.assertIn("user testing: 2 rounds, 3 flow groups, 13.00 USD-eq", text)

    def test_a_window_without_a_user_testing_session_says_none(self):
        s = Scratch()
        try:
            s.turn(at("00:00:10"), MISSION, FABLE, i=1)
            text = s.run().text
        finally:
            s.close()
        self.assertEqual(table(text, "User testing rounds")[1][0]["Round"], "none")
        self.assertIn("user testing: none", text)

    def test_the_caption_says_where_rounds_and_groups_come_from(self):
        self.assertIn("A round is a root session of the feature `user-testing-validator-*`", self.text)
        self.assertIn("callingToolUseId", self.text)


if __name__ == "__main__":
    unittest.main()
