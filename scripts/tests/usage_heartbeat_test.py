#!/usr/bin/env python3
"""Tests for the heartbeat and boundary split of the orchestrator turns in scripts/usage-report.py (owner item 1 of the M5
boundary, inbox 1350), on a scratch copy of the usage fixture (usage_scratch.py) on 2026-10-26.

The orchestrator transcript holds two heartbeats (a 200 s and a 240 s sleep), a boundary turn, two near misses (the sleep and
the reads beside a second call; the sleep without the reads) and one log turn with no transcript record.
"""

import unittest

from usage_scratch import FABLE, MISSION, Scratch, at, table

HEARTBEAT = """// @script: {"startInBackground": true}
await new Promise(r => setTimeout(r, %d));
const [inbox, prog, hand] = await Promise.allSettled([
  tools.LS({ directory_path: `${MD}/owner-feedback/inbox` }),
  tools.Read({ file_path: `${MD}/contract-work/progress.md` }),
  tools.LS({ directory_path: `${MD}/handoffs` }),
]);"""
SLEEP_ONLY = """// @script: {"startInBackground": true}
await new Promise(r => setTimeout(r, 200000));
await tools.Execute({ command: 'make check' });"""
BOUNDARY = "const r = await tools.StartMissionRun({ message: 'next feature' });"


def script(record_id, ts, source, extra=()):
    blocks = [{"type": "tool_use", "id": "t" + record_id, "name": "Script", "input": {"script": source}}]
    blocks += [{"type": "tool_use", "id": "x%s%d" % (record_id, i), "name": name, "input": {}} for i, name in enumerate(extra)]
    return {"id": record_id, "timestamp": ts, "message": {"role": "assistant", "content": blocks}}


def scenario(s):
    for hh in ("00:10:00", "00:20:00", "00:30:00", "00:40:00", "00:50:00", "01:00:00"):
        s.turn(at(hh), MISSION, FABLE, i=1000, o=100)
    s.transcript(MISSION, [
        script("hb200", at("00:10:00"), HEARTBEAT % 200000),
        script("hb240", at("00:20:00"), HEARTBEAT % 240000),
        script("b1", at("00:30:00"), BOUNDARY),
        script("miss-calls", at("00:40:00"), HEARTBEAT % 200000, extra=("Task",)),
        script("miss-reads", at("00:50:00"), SLEEP_ONLY),
    ])
    s.progress("2026-10-26T00:35:00.000Z", "worker_completed", workerSessionId="aaaaaaaa-0000-4000-8000-0000000000c1", featureId="m9-x")
    s.progress("2026-10-26T00:55:00.000Z", "worker_completed", workerSessionId="aaaaaaaa-0000-4000-8000-0000000000c2", featureId="m9-y")


class HeartbeatSplit(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.s = Scratch()
        scenario(cls.s)
        cls.text = cls.s.run().text

    @classmethod
    def tearDownClass(cls):
        cls.s.close()

    def rows(self):
        header, rows = table(self.text, "Orchestrator heartbeat and boundary turns")
        self.assertEqual(header, ["Kind", "Turns", "Input", "Output", "Cache read", "Cache write", "USD-eq", "Share of turns %"])
        return {r["Kind"]: r for r in rows}

    def test_the_orchestrator_row_splits_into_heartbeat_boundary_and_unmatched_turns(self):
        rows = self.rows()
        self.assertEqual([rows[k]["Turns"] for k in ("heartbeat", "boundary", "unmatched", "total")], ["2", "3", "1", "6"])
        self.assertEqual(rows["heartbeat"]["Input"], "2000")
        self.assertEqual(rows["total"]["Input"], "6000")
        self.assertEqual(rows["heartbeat"]["Share of turns %"], "33.3")

    def test_a_script_with_the_sleep_and_the_reads_beside_another_call_is_a_boundary_turn(self):
        self.assertIn("boundary turns per feature boundary: 3 / 2 = 1.5", self.text)

    def test_the_detector_rule_is_stated_in_the_report(self):
        self.assertIn("heartbeat detector: an orchestrator turn whose only tool call is a Script", self.text)
        self.assertIn("200000 ms (200 s)", self.text)
        self.assertIn("240000 ms (240 s)", self.text)
        self.assertIn("inbox", self.text.split("heartbeat detector:")[1].split("\n")[0])

    def test_one_matched_and_one_unmatched_example_are_pasted_from_the_transcript(self):
        self.assertIn("heartbeat example (matched): %s" % at("00:10:00"), self.text)
        self.assertIn("setTimeout(r, 200000)", self.text.split("heartbeat example (matched):")[1].split("boundary example")[0])
        unmatched = self.text.split("boundary example (not matched):")[1].split("\n\n")[0]
        self.assertIn(at("00:40:00"), unmatched)
        self.assertIn("2 tool calls", unmatched)


if __name__ == "__main__":
    unittest.main()
