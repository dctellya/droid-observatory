#!/usr/bin/env python3
"""Tests for scripts/usage.py over the redacted fixture in scripts/tests/fixtures/usage/.

The fixture is a hand-written excerpt shaped like the real Droid log, sidecars, orchestrator transcript,
progress log and gate records; ids are made up (11111111-... is the mission and its orchestrator). Every
expected number below is the arithmetic of the fixture lines and of fixtures/usage/prices.toml
(USD per million tokens: fable 10 / 50 / 0.25 / 12.5, opus 4 / 20 / 0.2 / 5, sonnet 2 / 10 / 0.1 / 2.5,
haiku 1 / 5 / 0.1 / 1.25 for input / output / cache read / cache write).
"""

import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "..", "usage.py")
FIX = os.path.join(HERE, "fixtures", "usage")

MISSION = "11111111-1111-4111-8111-111111111111"
SUBAGENT = "22222222-2222-4222-8222-222222222222"
WORKER = "33333333-3333-4333-8333-333333333333"
VALIDATOR = "44444444-4444-4444-8444-444444444444"
REVIEWER = "55555555-5555-4555-8555-555555555555"
SCRATCH = "66666666-6666-4666-8666-666666666666"
SCRATCH_WORKER = "77777777-7777-4777-8777-777777777777"

FROM = "2026-10-25T10:00:00Z"
NOW = "2026-10-25T11:00:00Z"
TO = "2026-10-25T10:59:59Z"

BEGIN = "<!-- usage:begin -->"
END = "<!-- usage:end -->"


def read(path):
    with open(path) as f:
        return f.read()


def run(extra=(), out=None, check=True, now=NOW):
    args = [
        sys.executable, SCRIPT,
        "--milestone", "m9",
        "--mission-dir", os.path.join(FIX, "mission"),
        "--mission-id", MISSION,
        "--logs-dir", os.path.join(FIX, "logs"),
        "--sessions-dir", os.path.join(FIX, "sessions"),
        "--prices", os.path.join(FIX, "prices.toml"),
        "--gate-dir", os.path.join(FIX, "gate"),
        "--from", FROM,
        "--now", now,
        "--out", out,
    ]
    args += list(extra)
    proc = subprocess.run(args, capture_output=True, text=True)
    if check and proc.returncode != 0:
        raise AssertionError("usage.py exit %d\nstdout: %s\nstderr: %s" % (proc.returncode, proc.stdout, proc.stderr))
    return proc


def table(text, heading):
    """Rows of the pipe table under `### heading` as dicts keyed by the header cells."""
    lines = text.splitlines()
    start = lines.index("### " + heading)
    rows = []
    header = None
    for line in lines[start + 1:]:
        if line.startswith("#"):
            break
        if not line.startswith("|"):
            if header is not None:
                break
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if header is None:
            header = cells
        elif set("".join(cells)) <= set("-: "):
            continue
        else:
            rows.append(dict(zip(header, cells)))
    assert header is not None, "no table under ### " + heading
    return header, rows


class UsageReport(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.out = os.path.join(cls.tmp.name, "m9.md")
        run(["--cap", "orchestrator=70"], out=cls.out)
        cls.text = read(cls.out)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def role(self, name):
        _, rows = table(self.text, "Tokens per role")
        return next(r for r in rows if r["Role"] == name)

    def test_the_tables_sit_between_the_markers_and_name_the_window(self):
        self.assertEqual(self.text.count(BEGIN), 1)
        self.assertEqual(self.text.count(END), 1)
        self.assertLess(self.text.index(BEGIN), self.text.index(END))
        self.assertIn("generated: 2026-10-25T11:00:00Z", self.text)
        self.assertIn("window: 2026-10-25T10:00:00Z to 2026-10-25T10:59:59Z", self.text)

    def test_the_default_end_of_the_window_is_the_generation_time_minus_one_second(self):
        out = os.path.join(self.tmp.name, "default-to.md")
        run(out=out)
        text = read(out)
        self.assertIn("window: 2026-10-25T10:00:00Z to 2026-10-25T10:59:59Z", text)

    def test_the_default_start_of_the_window_is_the_first_worker_of_the_milestone(self):
        out = os.path.join(self.tmp.name, "default-from.md")
        proc = subprocess.run(
            [sys.executable, SCRIPT, "--milestone", "m9", "--mission-dir", os.path.join(FIX, "mission"),
             "--mission-id", MISSION, "--logs-dir", os.path.join(FIX, "logs"),
             "--sessions-dir", os.path.join(FIX, "sessions"), "--prices", os.path.join(FIX, "prices.toml"),
             "--gate-dir", os.path.join(FIX, "gate"), "--now", NOW, "--out", out],
            capture_output=True, text=True)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        text = read(out)
        self.assertIn("window: 2026-10-25T10:00:30Z to 2026-10-25T10:59:59Z", text)
        self.assertEqual(table(text, "Tokens per role")[1][0]["Role"], "orchestrator")

    def test_tokens_per_role_are_summed_from_the_log_and_priced(self):
        o = self.role("orchestrator")
        self.assertEqual(
            (o["Sessions"], o["Turns"], o["Input"], o["Output"], o["Cache read"], o["Cache write"], o["Reasoning"]),
            ("1", "6", "3600007", "360003", "14000005", "640000", "300"))
        self.assertEqual((o["USD-eq"], o["Unpriced turns"]), ("65.50", "1"))
        self.assertEqual((o["First ts"], o["Last ts"]), ("2026-10-25T10:00:01.000Z", "2026-10-25T10:09:00.000Z"))
        self.assertEqual(o["Source"], "droid-log-single.log")

        w = self.role("worker")
        self.assertEqual(
            (w["Sessions"], w["Turns"], w["Input"], w["Output"], w["Cache read"], w["Cache write"], w["USD-eq"]),
            ("1", "2", "1500000", "600000", "40000000", "2000000", "18.00"))

        v = self.role("validator")
        self.assertEqual(
            (v["Sessions"], v["Turns"], v["Input"], v["Output"], v["Cache read"], v["Cache write"], v["USD-eq"], v["Unpriced turns"]),
            ("2", "4", "100015", "80005", "6500005", "1100000", "8.80", "1"))

        sub = self.role("orchestrator subagent")
        self.assertEqual((sub["Sessions"], sub["Turns"], sub["USD-eq"]), ("1", "1", "2.01"))

        sc = self.role("self-check")
        self.assertEqual((sc["Sessions"], sc["Turns"], sc["USD-eq"]), ("2", "2", "2.30"))

        other = self.role("other")
        self.assertEqual((other["Sessions"], other["Turns"], other["Input"]), ("2", "2", "3"))

    def test_credits_come_from_the_sidecars_and_sit_in_their_own_section(self):
        text = self.text
        start = text.index("#### Credits by role")
        rows = [l for l in text[start:].splitlines()[2:10] if l.startswith("| ")]
        got = {r.split("|")[1].strip(): r.split("|")[2].strip() for r in rows}
        self.assertEqual(got["orchestrator"], "1234567")
        self.assertEqual(got["worker"], "333333")
        self.assertEqual(got["validator"], str(444444 + 5555))
        self.assertGreater(text.index("### Sidecar readings"), text.index("### Mission to date by role"))

    def test_the_role_comes_from_the_sidecar_tags_and_the_progress_log(self):
        _, rows = table(self.text, "Tokens per session")
        by_id = {r["Session"].strip("`"): r for r in rows}
        self.assertEqual(by_id[MISSION]["Role"], "orchestrator")
        self.assertEqual(by_id[SUBAGENT]["Role"], "orchestrator subagent")
        self.assertEqual(by_id[SUBAGENT]["Kind"], "subagent of " + MISSION[:8])
        self.assertEqual((by_id[WORKER]["Role"], by_id[WORKER]["Feature"]), ("worker", "m9-alpha"))
        self.assertEqual((by_id[VALIDATOR]["Role"], by_id[VALIDATOR]["Feature"]), ("validator", "scrutiny-validator-m9"))
        self.assertEqual((by_id[REVIEWER]["Role"], by_id[REVIEWER]["Kind"]), ("validator", "subagent of " + VALIDATOR[:8]))
        self.assertEqual(by_id[SCRATCH]["Role"], "self-check")
        self.assertEqual(by_id["99999999-9999-4999-8999-999999999999"]["Role"], "other")
        w = by_id[WORKER]
        self.assertEqual((w["Models"], w["Turns"], w["USD-eq"]), ("claude-sonnet-5-5", "2", "18.00"))
        self.assertEqual((w["First ts"], w["Last ts"]), ("2026-10-25T10:00:40.000Z", "2026-10-25T10:01:00.000Z"))
        self.assertNotIn("33333333-3333-4333-8333-33333333333b", by_id)

    def test_every_session_row_names_its_source_and_timestamps(self):
        _, rows = table(self.text, "Tokens per session")
        self.assertGreaterEqual(len(rows), 9)
        for r in rows:
            self.assertRegex(r["First ts"], r"^\d{4}-\d\d-\d\dT[\d:.]+Z$", r["Session"])
            self.assertRegex(r["Last ts"], r"^\d{4}-\d\d-\d\dT[\d:.]+Z$", r["Session"])
            self.assertIn("droid-log-single.log", r["Source"], r["Session"])

    def test_the_sources_table_lists_each_file_with_line_counts_and_timestamps(self):
        _, rows = table(self.text, "Sources")
        by_name = {r["Source"].strip("`"): r for r in rows}
        live = by_name["logs/droid-log-single.log"]
        self.assertEqual((live["Lines"], live["First timestamp"], live["Last timestamp"]),
                         ("22", "2026-10-25T10:00:00.000Z", "2026-10-25T10:50:00.000Z"))
        self.assertIn("unparsed 1", live["Note"])
        old = by_name["logs/droid-log-single.log.2026-10-24"]
        self.assertEqual((old["Lines"], old["First timestamp"]), ("3", "2026-10-24T22:59:59.000Z"))
        self.assertEqual(by_name["mission/progress_log.jsonl"]["Lines"], "10")
        sidecars = next(r for k, r in by_name.items() if "sidecars" in k)
        self.assertEqual(sidecars["Lines"], "8")

    def test_the_orchestrator_split_classes_each_turn_from_the_transcript(self):
        _, rows = table(self.text, "Orchestrator split")
        got = {(r["Class"], r["Model"]): r for r in rows}
        w = got[("waits", "claude-fable-5.1")]
        self.assertEqual((w["Turns"], w["Input"], w["Output"], w["Cache read"], w["Cache write"], w["USD-eq"]),
                         ("2", "3000000", "300000", "12000000", "240000", "51.00"))
        d = got[("decisions and boundary reports", "claude-fable-5.1")]
        self.assertEqual((d["Turns"], d["USD-eq"]), ("1", "12.00"))
        k = got[("decisions and boundary reports", "kimi-k3")]
        self.assertEqual((k["Turns"], k["Unpriced turns"]), ("1", "1"))
        n = got[("narration", "claude-fable-5.1")]
        self.assertEqual((n["Turns"], n["USD-eq"]), ("1", "2.50"))
        u = got[("unmatched", "claude-fable-5.1")]
        self.assertEqual(u["Turns"], "1")

    def test_the_count_of_re_issued_waits_is_a_stated_number(self):
        self.assertIn("re-issued waits: 2", self.text)
        self.assertIn("waits returning in under 60 s: 1", self.text)
        self.assertIn("WaitForScript calls in the window: 2", self.text)
        self.assertIn("orchestrator turns matched to the transcript: 5 of 6", self.text)

    def test_subagents_and_self_check_missions_have_their_own_lines(self):
        _, rows = table(self.text, "Subagents")
        got = {r["Caller role"]: r for r in rows}
        self.assertEqual((got["orchestrator"]["Sessions"], got["orchestrator"]["Turns"]), ("1", "1"))
        self.assertEqual((got["validator"]["Sessions"], got["validator"]["Turns"]), ("1", "2"))
        _, rows = table(self.text, "Self-check missions")
        self.assertEqual(len(rows), 1)
        r = rows[0]
        self.assertEqual((r["Mission"].strip("`"), r["Sessions"], r["Turns"], r["USD-eq"]), (SCRATCH, "2", "2", "2.30"))
        self.assertEqual(r["Models"], "claude-haiku-4-5-20251001, claude-opus-5-5")

    def test_model_fallback_lists_every_turn_on_a_non_declared_model(self):
        _, rows = table(self.text, "Model fallback")
        got = {(r["Session"].strip("`"), r["Model"]): r for r in rows}
        self.assertEqual(set(got), {(MISSION, "kimi-k3"), (REVIEWER, "kimi-k3"), (SCRATCH_WORKER, "claude-opus-5-5")})
        self.assertEqual(got[(MISSION, "kimi-k3")]["Role"], "orchestrator")
        self.assertEqual(got[(REVIEWER, "kimi-k3")]["Role"], "validator")
        self.assertEqual(got[(SCRATCH_WORKER, "claude-opus-5-5")]["Role"], "self-check")
        self.assertIn("model fallback: 3 turns on non-declared models, 2 router transitions", self.text)
        _, rows = table(self.text, "Model-Router transitions")
        self.assertEqual([(r["Time"], r["Role"], r["Source"], r["Reason"], r["From"], r["To"]) for r in rows], [
            ("2026-10-25T10:06:30.000Z", "orchestrator", "manual", "-", "claude-fable-5.1", "kimi-k3"),
            ("2026-10-25T10:30:30.000Z", "validator", "detector_recovery", "overage_reactive_402", "claude-opus-5-5", "kimi-k3"),
        ])

    def test_throttled_windows_pair_a_pause_with_its_resume(self):
        _, rows = table(self.text, "Throttled")
        self.assertEqual(len(rows), 2)
        t = rows[0]
        self.assertEqual((t["Pause"], t["Resume"], t["Seconds"], t["Reason"], t["Throttled"]),
                         ("2026-10-25T10:10:00.000Z", "2026-10-25T10:30:00.000Z", "1200.000", "unrecoverable_usage_402", "yes"))
        self.assertEqual(t["Source"], "mission/progress_log.jsonl:5, :7")
        u = rows[1]
        self.assertEqual((u["Throttled"], u["Seconds"]), ("no", "60.000"))
        self.assertIn("mission_run_started", u["Resume"])
        self.assertIn("throttled: 1200.000 s = 20.00 min", self.text)

    def test_gates_run_come_from_the_records_in_the_window(self):
        _, rows = table(self.text, "Gates")
        self.assertEqual([(r["Gate"], r["Tree"], r["Exit"], r["Seconds"], r["Started"], r["Session"].strip("`")) for r in rows], [
            ("check", "t1", "0", "60.000", "2026-10-25T10:55:00Z", WORKER),
            ("test", "t1", "0", "40.000", "2026-10-25T10:55:10Z", WORKER),
        ])
        self.assertIn("gate runs in the window: 2", self.text)
        self.assertIn("gate seconds run: 100.000", self.text)
        self.assertIn("served from cache: not recorded", self.text)

    def test_validation_overhead_is_the_validator_session_wall_time(self):
        _, rows = table(self.text, "Validation overhead")
        self.assertEqual(len(rows), 1)
        r = rows[0]
        self.assertEqual((r["Session"].strip("`"), r["Feature"], r["Minutes (log)"], r["Minutes (progress_log)"]),
                         (VALIDATOR, "scrutiny-validator-m9", "30.00", "36.00"))
        self.assertEqual((r["First turn"], r["Last turn"]), ("2026-10-25T10:20:00.000Z", "2026-10-25T10:50:00.000Z"))
        self.assertIn("validation overhead minutes (log): 30.00", self.text)

    def test_feature_size_counts_turns_and_rounds_per_feature(self):
        _, rows = table(self.text, "Feature size")
        got = {r["Feature"]: r for r in rows}
        self.assertEqual((got["m9-alpha"]["Rounds"], got["m9-alpha"]["Turns"], got["m9-alpha"]["Over 120 turns"]), ("2", "2", "no"))
        self.assertEqual((got["scrutiny-validator-m9"]["Rounds"], got["scrutiny-validator-m9"]["Turns"]), ("1", "2"))

    def test_mission_to_date_reads_the_whole_log_and_compares_with_the_caps(self):
        _, rows = table(self.text, "Mission to date by role")
        got = {r["Role"]: r for r in rows}
        o = got["orchestrator"]
        self.assertEqual((o["Turns"], o["Input"], o["USD-eq"], o["Cap USD-eq"], o["Over cap"]), ("7", "4600007", "75.50", "70", "yes"))
        self.assertEqual(o["First ts"], "2026-10-24T23:00:00.000Z")
        self.assertEqual((got["worker"]["Cap USD-eq"], got["worker"]["Over cap"]), ("-", "-"))
        self.assertEqual(got["other"]["Turns"], "3")


class UsageReportFile(unittest.TestCase):
    def test_a_rerun_with_the_same_bounds_differs_only_in_the_generation_line(self):
        with tempfile.TemporaryDirectory() as tmp:
            a, b = os.path.join(tmp, "a.md"), os.path.join(tmp, "b.md")
            run(["--to", TO], out=a, now="2026-10-25T11:00:00Z")
            run(["--to", TO], out=b, now="2026-10-25T11:05:00Z")
            la, lb = read(a).splitlines(), read(b).splitlines()
            self.assertEqual(len(la), len(lb))
            diff = [(x, y) for x, y in zip(la, lb) if x != y]
            self.assertEqual(len(diff), 1, diff)
            self.assertTrue(diff[0][0].startswith("generated: "))

    def test_lines_written_after_the_window_end_do_not_change_the_tables(self):
        with tempfile.TemporaryDirectory() as tmp:
            logs = os.path.join(tmp, "logs")
            shutil.copytree(os.path.join(FIX, "logs"), logs)
            a, b = os.path.join(tmp, "a.md"), os.path.join(tmp, "b.md")
            run(["--to", TO, "--logs-dir", logs], out=a)
            with open(os.path.join(logs, "droid-log-single.log"), "a") as f:
                f.write('[2026-10-25T11:30:00.000Z] INFO: [Agent] Streaming result | Context: {"inputTokens":5,"outputTokens":5,'
                        '"cacheReadInputTokens":5,"cachedTokensWritten":0,"tags":{"sessionId":"%s","modelId":"claude-fable-5.1"}}\n' % MISSION)
            run(["--to", TO, "--logs-dir", logs], out=b)
            self.assertEqual(read(a), read(b))

    def test_prose_outside_the_markers_survives_a_regeneration(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "m9.md")
            with open(out, "w") as f:
                f.write("# Usage m9\n\nIntegrator prose.\n\n%s\nold table\n%s\n\nMore prose.\n" % (BEGIN, END))
            run(out=out)
            text = read(out)
            self.assertTrue(text.startswith("# Usage m9\n\nIntegrator prose.\n\n" + BEGIN))
            self.assertTrue(text.endswith(END + "\n\nMore prose.\n"))
            self.assertNotIn("old table", text)
            self.assertIn("### Tokens per role", text)

    def test_a_file_without_markers_keeps_its_text_and_gets_the_block_appended(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "m9.md")
            with open(out, "w") as f:
                f.write("# Usage m9\n\nHand-written.\n")
            run(out=out)
            text = read(out)
            self.assertTrue(text.startswith("# Usage m9\n\nHand-written.\n"))
            self.assertEqual(text.count(BEGIN), 1)

    def test_a_missing_log_directory_fails_with_a_message(self):
        with tempfile.TemporaryDirectory() as tmp:
            proc = run(["--logs-dir", os.path.join(tmp, "nothing")], out=os.path.join(tmp, "x.md"), check=False)
            self.assertNotEqual(proc.returncode, 0)
            self.assertIn("no droid-log-single.log", proc.stderr)

    def test_the_cross_check_against_usage_from_log_is_identical(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "m9.md")
            run(["--cross-check"], out=out)
            self.assertIn("cross-check: identical", read(out))

    def test_the_script_imports_only_the_standard_library(self):
        src = read(SCRIPT)
        mods = set(re.findall(r"^(?:import|from)\s+([A-Za-z_][\w]*)", src, re.M))
        allowed = {"argparse", "glob", "json", "os", "re", "subprocess", "sys", "tomllib", "datetime", "collections", "bisect"}
        self.assertLessEqual(mods, allowed, mods - allowed)


if __name__ == "__main__":
    unittest.main(verbosity=2)
