#!/usr/bin/env python3
"""Tests for the e2e lines of the usage report's testing section (owner inbox file
docs/owner-feedback/inbox/2026-10-29-1552-flaky-e2e-is-a-feature.md): a flaked run is a line with the spec and the cost of the
rerun, never a pass. Records are reports/e2e/<run>/record.json as scripts/e2e_run.py writes them; scratch day 2026-10-26.
"""

import json
import os
import unittest

from usage_scratch import Scratch, table


def attempt(n, outcome, seconds, **extra):
    return dict(attempt=n, outcome=outcome, seconds=seconds, exit_code=0 if outcome == "passed" else 1, **extra)


def record(run, outcome, started, attempts, **extra):
    r = {"run": run, "outcome": outcome, "started_at": started, "finished_at": started, "attempts": attempts,
         "first_failure": "reports/e2e/%s/first-failure" % run}
    r.update(extra)
    return r


FLAKED = record("2026-10-26", "flaked", "2026-10-26T03:00:00Z",
                [attempt(1, "failed", 104.25, failed_specs=["specs/gate.e2e.ts", "specs/panel.e2e.ts"]),
                 attempt(2, "passed", 41.5)],
                flaked_specs=["specs/gate.e2e.ts", "specs/panel.e2e.ts"], rerun_seconds=41.5)
RED = record("2026-10-26-b", "failed", "2026-10-26T04:00:00Z",
             [attempt(1, "failed", 90.0, failed_specs=["specs/stall.e2e.ts"]),
              attempt(2, "failed", 30.0, failed_specs=["specs/stall.e2e.ts"])],
             failed_specs=["specs/stall.e2e.ts"], rerun_seconds=30.0)
GREEN = record("2026-10-26-c", "passed", "2026-10-26T05:00:00Z", [attempt(1, "passed", 120.0)])
OUTSIDE = record("2026-10-27", "flaked", "2026-10-27T03:00:00Z",
                 [attempt(1, "failed", 10.0, failed_specs=["specs/panel.e2e.ts"]), attempt(2, "passed", 7.0)],
                 flaked_specs=["specs/panel.e2e.ts"], rerun_seconds=7.0)


def report(records):
    s = Scratch()
    try:
        e2e = os.path.join(s.root, "e2e")
        for r in records:
            os.makedirs(os.path.join(e2e, r["run"]))
            with open(os.path.join(e2e, r["run"], "record.json"), "w") as f:
                json.dump(r, f)
        return s.run(["--e2e-dir", e2e]).text
    finally:
        s.close()


STAMPED_FLAKE = record("2026-10-26/030000", "flaked", "2026-10-26T03:00:00Z",
                       [attempt(1, "failed", 104.25, failed_specs=["specs/panel.e2e.ts"]), attempt(2, "passed", 41.5)],
                       flaked_specs=["specs/panel.e2e.ts"], rerun_seconds=41.5)
STAMPED_GREEN = record("2026-10-26/051500", "passed", "2026-10-26T05:15:00Z", [attempt(1, "passed", 120.0)])


def report_with_latest(records, latest):
    """Run directories as `make e2e` leaves them, plus the `latest` symlink at reports/e2e/latest."""
    s = Scratch()
    try:
        e2e = os.path.join(s.root, "e2e")
        for r in records:
            os.makedirs(os.path.join(e2e, r["run"]))
            with open(os.path.join(e2e, r["run"], "record.json"), "w") as f:
                json.dump(r, f)
        os.symlink(latest, os.path.join(e2e, "latest"))
        return s.run(["--e2e-dir", e2e]).text
    finally:
        s.close()


class E2eRunDirectories(unittest.TestCase):
    def test_a_flaked_run_stays_a_flake_line_after_a_later_green_run_the_same_day(self):
        text = report([STAMPED_FLAKE, STAMPED_GREEN])
        self.assertIn("e2e flake: 2026-10-26/030000: specs/panel.e2e.ts; rerun 41.50 s (first attempt 104.25 s)", text)
        self.assertIn("e2e runs: 2, passed 1, flaked 1, failed 0; rerun seconds: 41.50", text)

    def test_the_runs_table_lists_every_run_directory_with_its_own_source_path(self):
        _, rows = table(report([STAMPED_FLAKE, STAMPED_GREEN]), "E2E runs")
        self.assertEqual([(r["Run"], r["Outcome"]) for r in rows],
                         [("2026-10-26/030000", "flaked"), ("2026-10-26/051500", "passed")])
        self.assertTrue(rows[0]["Source"].endswith("2026-10-26/030000/record.json`"), rows[0]["Source"])

    def test_the_latest_symlink_is_not_a_run_of_its_own(self):
        text = report_with_latest([STAMPED_FLAKE, STAMPED_GREEN], "2026-10-26/051500")
        self.assertIn("e2e runs: 2, passed 1, flaked 1, failed 0; rerun seconds: 41.50", text)

    def test_a_record_written_before_run_stamps_is_still_counted(self):
        text = report([FLAKED, STAMPED_GREEN])
        self.assertIn("e2e runs: 2, passed 1, flaked 1, failed 0; rerun seconds: 41.50", text)
        self.assertIn("e2e flake: 2026-10-26:", text)


class E2eFlakes(unittest.TestCase):
    def test_a_flaked_run_is_a_line_with_its_specs_and_the_cost_of_the_rerun(self):
        text = report([FLAKED])
        self.assertIn("e2e flake: 2026-10-26: specs/gate.e2e.ts, specs/panel.e2e.ts; rerun 41.50 s (first attempt 104.25 s)", text)

    def test_the_runs_table_keeps_flaked_distinct_from_passed_and_failed(self):
        text = report([FLAKED, RED, GREEN])
        _, rows = table(text, "E2E runs")
        self.assertEqual({r["Run"]: r["Outcome"] for r in rows},
                         {"2026-10-26": "flaked", "2026-10-26-b": "failed", "2026-10-26-c": "passed"})

    def test_the_summary_counts_each_outcome_and_the_seconds_the_reruns_cost(self):
        text = report([FLAKED, RED, GREEN])
        self.assertIn("e2e runs: 3, passed 1, flaked 1, failed 1; rerun seconds: 71.50", text)

    def test_only_a_flaked_run_gets_a_flake_line(self):
        text = report([RED, GREEN])
        self.assertNotIn("e2e flake:", text)

    def test_a_run_outside_the_window_is_not_counted(self):
        text = report([FLAKED, OUTSIDE])
        self.assertIn("e2e runs: 1, passed 0, flaked 1, failed 0; rerun seconds: 41.50", text)

    def test_no_record_in_the_window_says_none(self):
        text = report([])
        self.assertEqual(table(text, "E2E runs")[1][0]["Run"], "none")
        self.assertIn("e2e runs: none", text)

    def test_the_section_says_flaked_is_green_and_where_the_evidence_is(self):
        text = report([FLAKED])
        self.assertIn("`flaked` is a first attempt that failed and one rerun that passed", text)
        self.assertIn("reports/e2e/<date>/<HHMMSS>/first-failure/", text)


if __name__ == "__main__":
    unittest.main()
