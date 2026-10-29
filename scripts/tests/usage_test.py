#!/usr/bin/env python3
"""Tests for scripts/usage-report.py over the redacted fixture in scripts/tests/fixtures/usage/.

The fixture is a hand-written excerpt shaped like the real Droid log, sidecars, orchestrator transcript,
progress log and gate records; ids are made up (11111111-... is the mission and its orchestrator). Every
expected number below is the arithmetic of the fixture lines and of fixtures/usage/prices.toml
(USD per million tokens: fable 10 / 50 / 0.25 / 12.5, opus 4 / 20 / 0.2 / 5, sonnet 2 / 10 / 0.1 / 2.5,
haiku 1 / 5 / 0.1 / 1.25 for input / output / cache read / cache write).
"""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "..", "usage-report.py")
FIX = os.path.join(HERE, "fixtures", "usage")

MISSION = "11111111-1111-4111-8111-111111111111"
SUBAGENT = "22222222-2222-4222-8222-222222222222"
WORKER = "33333333-3333-4333-8333-333333333333"
VALIDATOR = "44444444-4444-4444-8444-444444444444"
REVIEWER = "55555555-5555-4555-8555-555555555555"
SCRATCH = "66666666-6666-4666-8666-666666666666"
SCRATCH_WORKER = "77777777-7777-4777-8777-777777777777"

GATE_LABEL = "scripts/tests/fixtures/usage/gate"
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
        raise AssertionError("usage-report.py exit %d\nstdout: %s\nstderr: %s" % (proc.returncode, proc.stdout, proc.stderr))
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

    def bullet(self, text, label):
        return next(l for l in text.splitlines() if l.startswith("- " + label + ":"))

    def test_re_issued_waits_count_repeats_on_one_tool_call_id_and_name_their_definition(self):
        line = self.bullet(self.text, "re-issued waits")
        self.assertTrue(line.startswith("- re-issued waits: 1 ("), line)
        self.assertIn("same `toolCallId`", line)
        self.assertIn("not counted", line)

    def test_worker_pauses_are_the_kills_and_have_their_own_row_with_a_definition(self):
        line = self.bullet(self.text, "worker pauses (kills)")
        self.assertTrue(line.startswith("- worker pauses (kills): 1 ("), line)
        self.assertIn("`action` is `kill`", line)
        self.assertLess(self.text.index("- re-issued waits:"), self.text.index("- worker pauses (kills):"))

    def test_the_other_wait_counts_keep_their_numbers(self):
        self.assertIn("waits returning in under 60 s: 1", self.text)
        self.assertIn("WaitForScript calls in the window: 3", self.text)
        self.assertIn("orchestrator turns matched to the transcript: 5 of 6", self.text)

    def test_waits_on_different_ids_and_a_third_wait_on_one_id_are_counted_apart(self):
        with tempfile.TemporaryDirectory() as tmp:
            sessions = os.path.join(tmp, "sessions")
            shutil.copytree(os.path.join(FIX, "sessions"), sessions)
            path = os.path.join(sessions, "-home-user-proj", MISSION + ".jsonl")
            extra = (
                '{"type":"message","id":"a7","timestamp":"2026-10-25T10:08:40.000Z","message":{"role":"assistant","content":'
                '[{"type":"tool_use","id":"toolu_w3","name":"WaitForScript","input":{"toolCallId":"toolu_bg1","action":"wait"}}]}}\n'
                '{"type":"message","id":"a8","timestamp":"2026-10-25T10:08:50.000Z","message":{"role":"assistant","content":'
                '[{"type":"tool_use","id":"toolu_w4","name":"WaitForScript","input":{"toolCallId":"toolu_bg3"}}]}}\n'
                '{"type":"message","id":"a9","timestamp":"2026-10-25T10:08:55.000Z","message":{"role":"assistant","content":'
                '[{"type":"tool_use","id":"toolu_k2","name":"WaitForScript","input":{"toolCallId":"toolu_bg1","action":"kill"}}]}}\n')
            with open(path, "a") as f:
                f.write(extra)
            out = os.path.join(tmp, "m9.md")
            run(["--sessions-dir", sessions], out=out)
            text = read(out)
            self.assertTrue(self.bullet(text, "re-issued waits").startswith("- re-issued waits: 2 ("))
            self.assertTrue(self.bullet(text, "worker pauses (kills)").startswith("- worker pauses (kills): 2 ("))
            self.assertIn("WaitForScript calls in the window: 6", text)

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

    def test_gate_runs_come_from_the_records_in_the_window(self):
        _, rows = table(self.text, "Gate runs")
        self.assertEqual([(r["Gate"], r["Tree"], r["Exit"], r["Seconds"], r["Started"], r["Session"].strip("`")) for r in rows], [
            ("check", "t1", "0", "60.000", "2026-10-25T10:55:00Z", WORKER),
            ("test", "t1", "0", "40.000", "2026-10-25T10:55:10Z", WORKER),
        ])
        self.assertIn("gate runs in the window: 2", self.text)
        self.assertIn("gate seconds run: 100.000", self.text)

    def test_the_gates_table_counts_runs_and_cache_hits_per_gate_and_in_total(self):
        header, rows = table(self.text, "Gates")
        self.assertEqual(header, ["Kind", "Gate", "Command", "Count", "Seconds", "Minutes", "First", "Last", "Source"])
        got = {(r["Kind"], r["Gate"]): r for r in rows}
        self.assertEqual(len(got), len(rows))
        cells = lambda k, g: tuple(got[(k, g)][c] for c in ("Command", "Count", "Seconds", "Minutes", "First", "Last", "Source"))
        self.assertEqual(cells("gates run", "check"), (
            "`make check`", "1", "60.000", "1.00", "2026-10-25T10:55:00Z", "2026-10-25T10:56:00Z", "`%s/*/check.json`" % GATE_LABEL))
        self.assertEqual(cells("gates run", "test")[1:4], ("1", "40.000", "0.67"))
        self.assertEqual(cells("gates run", "all"), (
            "-", "2", "100.000", "1.67", "2026-10-25T10:55:00Z", "2026-10-25T10:56:00Z", "`%s/*/*.json`" % GATE_LABEL))
        self.assertEqual(cells("gates served from cache", "check"), (
            "`make check`", "2", "120.000", "2.00", "2026-10-25T10:57:00Z", "2026-10-25T10:57:30Z", "`%s/hits.log`" % GATE_LABEL))
        self.assertEqual(cells("gates served from cache", "test")[1:6], ("1", "40.000", "0.67", "2026-10-25T10:58:00Z", "2026-10-25T10:58:00Z"))
        self.assertEqual(cells("gates served from cache", "all")[1:4], ("3", "160.000", "2.67"))
        self.assertIn("gates served from cache: 3, minutes saved: 2.67", self.text)
        self.assertNotIn("not recorded", self.text)

    def gate_report(self, records):
        """Report text over a copy of the gate fixtures plus `records` ({name: json fields} under tree `tx`)."""
        with tempfile.TemporaryDirectory() as tmp:
            gate = os.path.join(tmp, "gate")
            shutil.copytree(os.path.join(FIX, "gate"), gate)
            os.makedirs(os.path.join(gate, "tx"))
            for name, fields in records.items():
                base = {"name": name, "tree": "tx", "tool": "fixture", "command": "make " + name, "exit_code": 0,
                        "summary": "ok", "seconds": 1.0, "session": WORKER}
                base.update(fields)
                with open(os.path.join(gate, "tx", name + ".json"), "w") as f:
                    json.dump({k: v for k, v in base.items() if v is not None}, f)
            out = os.path.join(tmp, "m9.md")
            run(["--gate-dir", gate, "--to", TO], out=out)
            return read(out)

    def test_a_gate_run_that_straddles_either_edge_of_the_window_is_in_the_table(self):
        text = self.gate_report({
            "ends-after": {"started_at": "2026-10-25T10:59:30Z", "finished_at": "2026-10-25T11:00:30Z"},
            "starts-before": {"started_at": "2026-10-25T09:59:30Z", "finished_at": "2026-10-25T10:00:30Z"},
            "no-start": {"started_at": None, "finished_at": "2026-10-25T10:30:00Z"},
            "after": {"started_at": "2026-10-25T11:00:10Z", "finished_at": "2026-10-25T11:00:40Z"},
            "before": {"started_at": "2026-10-25T09:58:00Z", "finished_at": "2026-10-25T09:59:50Z"},
        })
        _, rows = table(text, "Gate runs")
        names = {r["Gate"] for r in rows}
        self.assertEqual(names, {"check", "test", "ends-after", "starts-before", "no-start"})
        _, rows = table(text, "Gates")
        self.assertEqual({r["Gate"] for r in rows if r["Kind"] == "gates run"}, names | {"all"})
        self.assertIn("gate runs in the window: 5", text)

    def test_the_gates_caption_states_the_window_rule(self):
        caption = next(l for l in self.text.splitlines() if l.startswith("Gate window rule:"))
        self.assertIn("`started_at` < `--to` or `finished_at` <= `--to`", caption)
        self.assertIn("did not finish before the window start", caption)
        self.assertIn("started at or after `--to` is not", caption)

    def test_a_report_without_a_hit_log_states_zero_hits(self):
        with tempfile.TemporaryDirectory() as tmp:
            gate = os.path.join(tmp, "gate")
            shutil.copytree(os.path.join(FIX, "gate"), gate)
            os.remove(os.path.join(gate, "hits.log"))
            out = os.path.join(tmp, "m9.md")
            run(["--gate-dir", gate], out=out)
            text = read(out)
            self.assertIn("gates served from cache: 0, minutes saved: 0.00", text)
            _, rows = table(text, "Gates")
            self.assertEqual([r["Count"] for r in rows if r["Kind"] == "gates served from cache"], ["0"])

    def test_the_sidecar_section_states_the_mtimes_it_read(self):
        text = self.text[self.text.index("### Sidecar readings"):]
        sidecar_dir = os.path.join(FIX, "sessions", "-home-user-proj")
        mtimes = {}
        for name in os.listdir(sidecar_dir):
            if name.endswith(".settings.json"):
                mtimes[name[:-len(".settings.json")]] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(os.path.getmtime(os.path.join(sidecar_dir, name))))
        oldest, newest = min(mtimes.values()), max(mtimes.values())
        self.assertIn("sidecars read: %d, oldest mtime %s, newest mtime %s" % (len(mtimes), oldest, newest), text)
        lines = text.splitlines()
        per_session = lines.index("#### Sidecar totals per session (cumulative)")
        header = [c.strip() for c in lines[per_session + 2].strip().strip("|").split("|")]
        self.assertIn("Modified", header)
        first = [c.strip() for c in lines[per_session + 4].strip().strip("|").split("|")]
        self.assertEqual(first[header.index("Modified")], mtimes[first[0].strip("`")])

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

    def test_feature_size_rows_carry_skill_timestamps_and_sources(self):
        header, rows = table(self.text, "Feature size")
        for col in ("Skill", "First ts", "Last ts", "Source"):
            self.assertIn(col, header)
        got = {r["Feature"]: r for r in rows}
        _, sessions = table(self.text, "Tokens per session")
        w = next(r for r in sessions if r["Session"].strip("`") == WORKER)
        a = got["m9-alpha"]
        self.assertEqual((a["Skill"], a["First ts"], a["Last ts"]), ("infra-worker", w["First ts"], w["Last ts"]))
        self.assertIn("droid-log-single.log", a["Source"])
        self.assertIn("features.json", a["Source"])
        self.assertIn("progress_log.jsonl", a["Source"])
        self.assertEqual(got["scrutiny-validator-m9"]["Skill"], "-")

    def test_the_sources_table_lists_features_json_with_its_feature_count(self):
        _, rows = table(self.text, "Sources")
        by_name = {r["Source"].strip("`"): r for r in rows}
        self.assertEqual(by_name["mission/features.json"]["Lines"], "2")

    def test_subagent_self_check_and_mission_to_date_rows_name_their_source_file(self):
        for heading in ("Subagents", "Self-check missions", "Mission to date by role"):
            header, rows = table(self.text, heading)
            self.assertIn("Source", header, heading)
            for r in rows:
                if r.get("Sessions") == "0":
                    continue
                self.assertIn("droid-log-single.log", r["Source"], "%s %s" % (heading, r))

    def test_mission_to_date_reads_the_whole_log_and_compares_with_the_caps(self):
        _, rows = table(self.text, "Mission to date by role")
        got = {r["Role"]: r for r in rows}
        o = got["orchestrator"]
        self.assertEqual((o["Turns"], o["Input"], o["USD-eq"], o["Cap USD-eq"], o["Over cap"]), ("7", "4600007", "75.50", "70", "yes"))
        self.assertEqual(o["First ts"], "2026-10-24T23:00:00.000Z")
        self.assertEqual((got["worker"]["Cap USD-eq"], got["worker"]["Over cap"]), ("-", "-"))
        self.assertEqual(got["other"]["Turns"], "3")


class UsageReportMilestoneCap(unittest.TestCase):
    def report(self, cap):
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "m9.md")
            run(["--cap", "milestone=" + cap, "--cap", "orchestrator=70"], out=out)
            return read(out)

    def test_a_milestone_cap_adds_a_total_row_with_over_cap(self):
        for cap, want in (("5", "yes"), ("110", "no")):
            text = self.report(cap)
            _, roles = table(text, "Tokens per role")
            window_total = next(r for r in roles if r["Role"] == "total")
            _, rows = table(text, "Mission to date by role")
            got = {r["Role"]: r for r in rows}
            m = got["milestone window total"]
            self.assertEqual((m["USD-eq"], m["Turns"]), (window_total["USD-eq"], window_total["Turns"]))
            self.assertEqual((m["Cap USD-eq"], m["Over cap"]), (cap, want))
            t = got["total"]
            self.assertEqual(t["Turns"], str(sum(int(r["Turns"]) for k, r in got.items() if k not in ("total", "milestone window total"))))
            self.assertEqual((t["Cap USD-eq"], t["Over cap"]), ("-", "-"))
            self.assertIn("droid-log-single.log", m["Source"])


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

    def test_a_line_logged_out_of_order_at_the_window_end_is_still_counted(self):
        # The real log is not time-ordered (lines up to 48 s backwards): a line past --to is skipped, not a stop.
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "m9.md")
            run(["--cross-check", "--logs-dir", os.path.join(FIX, "logs-out-of-order")], out=out)
            text = read(out)
            _, rows = table(text, "Tokens per role")
            o = next(r for r in rows if r["Role"] == "orchestrator")
            self.assertEqual((o["Turns"], o["Input"]), ("2", "3000"))
            self.assertEqual(o["Last ts"], "2026-10-25T10:59:55.000Z")
            self.assertIn("cross-check: identical", text)

    def test_help_states_when_a_rerun_is_byte_for_byte_reproducible(self):
        proc = subprocess.run([sys.executable, SCRIPT, "--help"], capture_output=True, text=True)
        self.assertEqual(proc.returncode, 0)
        flat = " ".join(proc.stdout.split())
        self.assertIn("--to at least 5 minutes before the generation time", flat)
        self.assertIn("Sidecar readings can move while a session is live", flat)

    def test_the_script_imports_only_the_standard_library(self):
        src = read(SCRIPT)
        mods = set(re.findall(r"^(?:import|from)\s+([A-Za-z_][\w]*)", src, re.M))
        allowed = {"argparse", "glob", "json", "os", "re", "subprocess", "sys", "tomllib", "datetime", "collections", "bisect"}
        self.assertLessEqual(mods, allowed, mods - allowed)


if __name__ == "__main__":
    unittest.main(verbosity=2)
