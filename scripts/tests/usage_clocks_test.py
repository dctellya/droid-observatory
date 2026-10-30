#!/usr/bin/env python3
"""Tests for idle time booked by cause (usage limit, owner wait, runner idle), the three clocks, the previous milestone recomputed
beside its recorded figures and the recorded Ratios rows marked not comparable (owner items 12 and 15 of the M5 boundary,
inbox 1350), on a scratch copy of the usage fixture (usage_scratch.py) on 2026-10-26.
"""

import os
import unittest

from usage_scratch import FABLE, MISSION, Scratch, at, table

KIMI = "kimi-k3"


def router(s, ts, source, reason, before, after):
    ctx = {"sessionId": MISSION, "source": source, "previousModelId": before, "modelId": after, "slot": "main"}
    if reason:
        ctx["reason"] = reason
    s.raw_log(ts, "INFO", "[Model-Router] Model transition", ctx)


def text_record(rid, ts, role, text):
    return {"id": rid, "timestamp": ts, "message": {"role": role, "content": [{"type": "text", "text": text}]}}


def limit_hold(s):
    """A hold the platform paused with no reason: the router fell back at 00:09:50 and the owner restored the model at 02:30:00."""
    s.turn(at("00:00:10"), MISSION, FABLE, i=1)
    s.turn(at("00:10:00"), MISSION, FABLE, i=1)
    router(s, at("00:09:50"), "detector_recovery", "overage_proactive", FABLE, KIMI)
    s.progress("2026-10-26T00:12:00.000Z", "mission_paused")
    router(s, at("02:30:00"), "manual", None, KIMI, FABLE)
    s.progress("2026-10-26T02:40:00.000Z", "mission_resumed")
    s.turn(at("02:41:00"), MISSION, FABLE, i=1)


class IdleByCause(unittest.TestCase):
    def test_a_hold_the_router_fell_back_for_is_a_usage_limit_stop_not_an_owner_wait(self):
        s = Scratch()
        try:
            limit_hold(s)
            text = s.run().text
        finally:
            s.close()
        _, rows = table(text, "Idle gaps")
        self.assertEqual([(r["Cause"], r["Minutes"]) for r in rows], [("usage limit", "148.00")])
        self.assertIn("Model-Router detector_recovery overage_proactive", rows[0]["Evidence"])
        _, by_cause = table(text, "Idle time by cause")
        got = {r["Cause"]: (r["Gaps"], r["Minutes"]) for r in by_cause}
        self.assertEqual(got["usage limit"], ("1", "148.00"))
        self.assertEqual(got["owner"], ("0", "0.00"))

    def test_a_gap_the_owner_answered_in_chat_is_an_owner_wait_with_the_question_as_evidence(self):
        s = Scratch()
        try:
            s.turn(at("03:00:00"), MISSION, FABLE, i=1)
            s.turn(at("03:40:05"), MISSION, FABLE, i=1)
            s.transcript(MISSION, [
                text_record("q", at("03:00:10"), "assistant", "The money figure forces a stop here; I need your decisions."),
                text_record("r", at("03:20:00"), "user", "<system-reminder>REMINDER: Your TodoWrite plan has open items</system-reminder>"),
                text_record("s", at("03:30:00"), "user", "Script Output from toolu_x heartbeat inbox (31) new: []"),
                text_record("a", at("03:39:30"), "user", "Owner answers, M5 at the 150 line. 1. Money: raise the cap to 190")])
            text = s.run().text
        finally:
            s.close()
        _, rows = table(text, "Idle gaps")
        self.assertEqual([(r["Cause"], r["Minutes"]) for r in rows], [("owner", "40.08")])
        self.assertIn("Owner answers, M5 at the 150 line", rows[0]["Evidence"])
        self.assertIn("I need your decisions", rows[0]["Evidence"])
        self.assertNotIn("REMINDER", rows[0]["Evidence"])

    def test_a_pause_with_no_owner_evidence_is_runner_idle(self):
        s = Scratch()
        try:
            s.turn(at("04:00:00"), MISSION, FABLE, i=1)
            s.progress("2026-10-26T04:01:00.000Z", "mission_paused")
            s.progress("2026-10-26T04:50:00.000Z", "mission_run_started")
            s.turn(at("04:51:00"), MISSION, FABLE, i=1)
            text = s.run().text
        finally:
            s.close()
        _, rows = table(text, "Idle gaps")
        self.assertEqual([r["Cause"] for r in rows], ["runner idle"])
        self.assertIn("mission_paused", rows[0]["Evidence"])

    def test_the_summary_lists_the_three_causes_beside_host_and_unexplained(self):
        s = Scratch()
        try:
            limit_hold(s)
            text = s.run().text
        finally:
            s.close()
        _, rows = table(text, "Idle time by cause")
        self.assertEqual([r["Cause"] for r in rows], ["usage limit", "host", "owner", "runner idle", "unexplained", "total"])
        self.assertIn("booked whole to the first cause", text)


class Clocks(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.s = Scratch()
        limit_hold(cls.s)
        for hh in ("02:55:00", "03:00:00", "03:40:05"):
            cls.s.turn(at(hh), MISSION, FABLE, i=1)
        cls.s.transcript(MISSION, [text_record("a", at("03:39:30"), "user", "Owner answers: go")])
        cls.text = cls.s.run(end=at("05:00:00")[:-5] + "Z", start="2026-10-26T00:00:00Z").text

    @classmethod
    def tearDownClass(cls):
        cls.s.close()

    def test_wall_cycle_and_system_time_are_three_rows_with_the_pauses_listed_by_cause(self):
        header, rows = table(self.text, "Clocks")
        self.assertEqual(header, ["Clock", "Minutes", "Hours", "Definition"])
        got = {r["Clock"]: (r["Minutes"], r["Hours"]) for r in rows}
        self.assertEqual(got["wall"], ("300.00", "5:00"))
        self.assertEqual(got["held, usage-limit stop"], ("148.00", "2:28"))
        self.assertEqual(got["cycle"], ("152.00", "2:32"))
        self.assertEqual(got["owner wait"], ("40.08", "0:40"))
        self.assertEqual(got["runner idle"], ("0.00", "0:00"))
        self.assertEqual(got["system"], ("111.92", "1:52"))

    def test_the_definitions_name_what_each_clock_answers(self):
        self.assertIn("cycle = wall less the usage-limit stops", self.text)
        self.assertIn("system = cycle less the owner waits and the runner idle", self.text)


class PreviousMilestone(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.s = Scratch()
        limit_hold(cls.s)
        cls.report = os.path.join(cls.s.tmp.name, "m8.md")
        with open(cls.report, "w") as f:
            f.write("# Usage: milestone m8\n\nwindow: 2026-10-26T00:00:00Z to 2026-10-26T06:00:00Z (`--to` is x)\n\n"
                    "### Idle time by cause\n\n| Cause | Gaps | Minutes |\n|---|---|---|\n| usage limit | 0 | 0.00 |\n| host | 0 | 0.00 |\n"
                    "| owner | 1 | 150.00 |\n| unexplained | 0 | 0.00 |\n| total | 1 | 150.00 |\n")
        cls.text = cls.s.run(["--previous", "m8", "--previous-report", cls.report]).text

    @classmethod
    def tearDownClass(cls):
        cls.s.close()

    def test_the_previous_window_is_recomputed_beside_the_recorded_idle_figures(self):
        header, rows = table(self.text, "Idle time by cause, m8 recorded and recomputed")
        self.assertEqual(header, ["Cause", "m8 (recorded)", "m8 (recomputed)"])
        got = {r["Cause"]: (r["m8 (recorded)"], r["m8 (recomputed)"]) for r in rows}
        self.assertEqual(got["owner"], ("150.00", "0.00"))
        self.assertEqual(got["usage limit"], ("0.00", "148.00"))
        self.assertEqual(got["runner idle"], ("-", "0.00"))

    def test_the_previous_milestone_is_recomputed_on_the_same_bases_in_the_ratios_table(self):
        header, rows = table(self.text, "Ratios")
        labels = [r["Milestone"] for r in rows]
        self.assertIn("m8 (recomputed, pinned bases)", labels)
        self.assertEqual(labels[-1], "m9 (this run)")
        self.assertIn("m8 bases: window 2026-10-26T00:00:00Z to 2026-10-26T06:00:00Z; w = worker cost", self.text)

    def test_a_recorded_row_on_other_bases_is_marked_not_comparable(self):
        s = Scratch()
        try:
            text = s.run().text
        finally:
            s.close()
        _, rows = table(text, "Ratios")
        labels = [r["Milestone"] for r in rows]
        self.assertEqual(labels[:2], ["m3 (recorded), not comparable", "m4 (recorded), not comparable"])
        self.assertNotIn("m8 (recomputed", " ".join(labels))


if __name__ == "__main__":
    unittest.main()
