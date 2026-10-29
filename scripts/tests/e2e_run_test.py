#!/usr/bin/env python3
"""Tests for scripts/e2e_run.py: the first-failure evidence, the one-rerun policy and the `flaked` outcome of `make e2e`
(owner inbox file docs/owner-feedback/inbox/2026-10-29-1552-flaky-e2e-is-a-feature.md).

The suite command is a stub (a Python script the test writes) that behaves as wdio does for this harness: it writes
junit.xml, a backend log, one screenshot, timing.jsonl and failed-specs.txt into the report directory, and fails or passes
by attempt as the test says.
"""

import json
import os
import re
import socket
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPTS = os.path.join(HERE, "..")
SCRIPT = os.path.join(SCRIPTS, "e2e_run.py")
REPO = os.path.join(SCRIPTS, "..")

SUITES = {"specs/panel.e2e.ts": "panel render", "specs/gate.e2e.ts": "gate lane"}

STUB = r'''
import json, os, sys
report, run = os.environ["E2E_REPORT_DIR"], os.environ["E2E_RUN_DIR"]
attempt = int(os.environ["E2E_ATTEMPT"])
fail_on = [int(x) for x in os.environ.get("STUB_FAIL_ATTEMPTS", "").split(",") if x]
specs = [a for i, a in enumerate(sys.argv) if i and sys.argv[i - 1] == "--spec"] or %(all)r
with open(os.path.join(os.environ["STUB_LOG"]), "a") as f:
    f.write(json.dumps({"attempt": attempt, "argv": sys.argv[1:], "run_dir_exists": os.path.isdir(run),
                        "first_failure_seen": os.path.isdir(os.path.join(report, "first-failure"))}) + "\n")
os.makedirs(os.path.join(run, "sessions"), exist_ok=True)
open(os.path.join(run, "observatory.sqlite"), "w").write("attempt %%d" %% attempt)
fails = attempt in fail_on
names = %(names)r
suites = []
for s in specs:
    bad = fails and s == "specs/gate.e2e.ts"
    case = '<testcase classname="%%s" name="case" time="1.0">%%s</testcase>' %% (names[s], '<failure message="card never reached Build"/>' if bad else "")
    props = '<properties>\n      <property name="specId" value="0"/>\n    </properties>'
    suites.append('<testsuite name="%%s" tests="1" failures="%%d" time="2.0">\n    %%s\n%%s</testsuite>' %% (names[s], 1 if bad else 0, props, case))
open(os.path.join(report, "junit.xml"), "w").write("<?xml version=\"1.0\"?>\n<testsuites>\n" + "\n".join(suites) + "\n</testsuites>\n")
open(os.path.join(report, "wdio-0-0.log"), "w").write("backend log of attempt %%d\n" %% attempt)
open(os.path.join(report, ("fail-" if fails else "pass-") + "case.png"), "w").write("png %%d" %% attempt)
open(os.path.join(report, "timing.jsonl"), "a").write('{"attempt": %%d}\n' %% attempt)
failed = os.path.join(report, "failed-specs.txt")
if fails:
    open(failed, "w").write("specs/gate.e2e.ts\n")
elif os.path.exists(failed):
    os.remove(failed)
sys.exit(1 if fails else 0)
''' % {"all": list(SUITES), "names": SUITES}


def read(path):
    with open(path) as f:
        return f.read()


def root_outcome(path):
    return re.search(r'<testsuites\b[^>]*\boutcome="([^"]*)"', read(path)).group(1)


def suites(path):
    """{suite name: its element text}; the stub's junit and the runner's are flat, and python's XML parser is not always
    loadable here (Homebrew python 3.14.7 fails to import pyexpat)."""
    return {re.search(r'name="([^"]*)"', s).group(1): s for s in re.findall(r"<testsuite\b.*?</testsuite>", read(path), re.S)}


class Harness:
    """A scratch report directory and run directory plus the stub suite command."""

    def __init__(self, fail_attempts=""):
        self.tmp = tempfile.TemporaryDirectory()
        self.report = os.path.join(self.tmp.name, "reports", "e2e", "2026-10-29")
        self.run_dir = os.path.join(self.tmp.name, "run")
        self.log = os.path.join(self.tmp.name, "stub-calls.jsonl")
        self.stub = os.path.join(self.tmp.name, "stub.py")
        with open(self.stub, "w") as f:
            f.write(STUB)
        os.makedirs(self.report)
        self.env = dict(os.environ, STUB_LOG=self.log, STUB_FAIL_ATTEMPTS=fail_attempts, GITHUB_TOKEN="ghp_not_to_be_recorded",
                        FORGE_LIVE="1")
        self.proc = None

    def run(self):
        self.proc = subprocess.run([sys.executable, SCRIPT, "run", "--report-dir", self.report, "--run-dir", self.run_dir, "--",
                                    sys.executable, self.stub], capture_output=True, text=True, env=self.env)
        return self.proc

    def calls(self):
        with open(self.log) as f:
            return [json.loads(line) for line in f]

    def record(self):
        with open(os.path.join(self.report, "record.json")) as f:
            return json.load(f)

    def close(self):
        self.tmp.cleanup()


class FirstFailure(unittest.TestCase):
    def setUp(self):
        self.h = Harness(fail_attempts="1")
        self.h.run()
        self.folder = os.path.join(self.h.report, "first-failure")

    def tearDown(self):
        self.h.close()

    def read(self, name):
        with open(os.path.join(self.folder, name)) as f:
            return f.read()

    def test_the_first_failing_attempt_leaves_its_evidence_before_the_rerun_starts(self):
        first, rerun = self.h.calls()
        self.assertFalse(first["first_failure_seen"])
        self.assertTrue(rerun["first_failure_seen"], "the rerun started before the evidence was written")
        for name in ("environment.json", "run-dir-listing.txt"):
            self.assertTrue(os.path.isfile(os.path.join(self.folder, name)), name)

    def test_the_backend_log_and_the_failing_screenshot_are_the_first_attempts_not_the_reruns(self):
        self.assertEqual(self.read("backend-logs/wdio-0-0.log"), "backend log of attempt 1\n")
        self.assertEqual(self.read("screenshots/fail-case.png"), "png 1")

    def test_the_run_directory_listing_names_the_files_the_failed_attempt_left(self):
        listing = self.read("run-dir-listing.txt")
        self.assertIn("observatory.sqlite", listing)
        self.assertIn("sessions", listing)

    def test_the_environment_names_the_run_the_ports_the_pids_and_whether_the_token_was_set(self):
        env = json.loads(self.read("environment.json"))
        self.assertEqual(env["E2E_RUN_DIR"], self.h.run_dir)
        self.assertEqual(env["E2E_ATTEMPT"], "1")
        self.assertEqual(env["GITHUB_TOKEN"], "set")
        self.assertEqual(env["FORGE_LIVE"], "1")
        self.assertIn("4445", env["ports"])
        self.assertIn("runner", env["pids"])

    def test_the_token_value_is_never_written_to_the_evidence(self):
        for root, _, files in os.walk(self.folder):
            for name in files:
                with open(os.path.join(root, name), errors="replace") as f:
                    self.assertNotIn("ghp_not_to_be_recorded", f.read(), name)


class FirstFailureCapture(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.report = os.path.join(self.tmp.name, "report")
        self.run_dir = os.path.join(self.tmp.name, "run")
        os.makedirs(self.report)
        os.makedirs(self.run_dir)
        self.addCleanup(self.tmp.cleanup)

    def capture(self, env, *extra):
        return subprocess.run([sys.executable, SCRIPT, "first-failure", "--report-dir", self.report, "--run-dir", self.run_dir,
                               *extra], capture_output=True, text=True, env=env)

    def test_a_listening_port_is_recorded_with_the_pid_that_holds_it(self):
        listener = socket.socket()
        self.addCleanup(listener.close)
        listener.bind(("127.0.0.1", 0))
        listener.listen(1)
        port = listener.getsockname()[1]
        proc = self.capture(dict(os.environ), "--port", str(port))
        self.assertEqual(proc.returncode, 0, proc.stderr)
        with open(os.path.join(self.report, "first-failure", "environment.json")) as f:
            env = json.load(f)
        self.assertIn(str(os.getpid()), env["ports"][str(port)])

    def test_unset_token_and_flag_read_unset(self):
        clean = {k: v for k, v in os.environ.items() if k not in ("GITHUB_TOKEN", "FORGE_LIVE")}
        self.assertEqual(self.capture(clean).returncode, 0)
        with open(os.path.join(self.report, "first-failure", "environment.json")) as f:
            env = json.load(f)
        self.assertEqual((env["GITHUB_TOKEN"], env["FORGE_LIVE"]), ("unset", "unset"))

    def test_a_second_failure_in_the_same_run_does_not_overwrite_the_first_ones_environment(self):
        self.capture(dict(os.environ, E2E_ATTEMPT="1"))
        self.capture(dict(os.environ, E2E_ATTEMPT="2"))
        with open(os.path.join(self.report, "first-failure", "environment.json")) as f:
            self.assertEqual(json.load(f)["E2E_ATTEMPT"], "1")


class RerunPolicy(unittest.TestCase):
    def outcome(self, fail_attempts):
        h = Harness(fail_attempts)
        self.addCleanup(h.close)
        proc = h.run()
        return h, proc

    def test_a_green_first_attempt_runs_once_and_is_passed(self):
        h, proc = self.outcome("")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(len(h.calls()), 1)
        self.assertEqual(h.record()["outcome"], "passed")
        self.assertFalse(os.path.exists(os.path.join(h.report, "first-failure")))

    def test_a_red_first_attempt_reruns_only_the_failed_specs_once_on_a_clean_run_directory(self):
        h, _ = self.outcome("1")
        calls = h.calls()
        self.assertEqual([c["attempt"] for c in calls], [1, 2])
        self.assertEqual(calls[1]["argv"], ["--spec", "specs/gate.e2e.ts"])
        self.assertFalse(calls[1]["run_dir_exists"], "the rerun did not start from a clean run directory")

    def test_a_green_rerun_is_flaked_never_passed_and_the_run_exits_green(self):
        h, proc = self.outcome("1")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        rec = h.record()
        self.assertEqual(rec["outcome"], "flaked")
        self.assertEqual([a["outcome"] for a in rec["attempts"]], ["failed", "passed"])
        self.assertEqual(rec["flaked_specs"], ["specs/gate.e2e.ts"])
        self.assertAlmostEqual(rec["rerun_seconds"], rec["attempts"][1]["seconds"])
        self.assertRegex(proc.stdout, r"e2e: flaked .*specs/gate\.e2e\.ts")

    def test_a_second_failure_is_a_plain_red_after_exactly_one_rerun(self):
        h, proc = self.outcome("1,2")
        self.assertEqual(len(h.calls()), 2)
        self.assertNotEqual(proc.returncode, 0)
        rec = h.record()
        self.assertEqual(rec["outcome"], "failed")
        self.assertEqual([a["outcome"] for a in rec["attempts"]], ["failed", "failed"])
        self.assertEqual(rec["failed_specs"], ["specs/gate.e2e.ts"])
        self.assertRegex(proc.stdout, r"e2e: failed")

    def test_the_record_keeps_both_attempts_with_wall_time_and_the_first_failure_folder(self):
        h, _ = self.outcome("1")
        rec = h.record()
        self.assertEqual([a["attempt"] for a in rec["attempts"]], [1, 2])
        self.assertEqual(rec["attempts"][0]["failed_specs"], ["specs/gate.e2e.ts"])
        for a in rec["attempts"]:
            self.assertGreaterEqual(a["seconds"], 0)
        self.assertRegex(rec["started_at"], r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$")
        self.assertTrue(os.path.isdir(os.path.join(h.report, "first-failure")))
        self.assertTrue(rec["first_failure"].endswith("first-failure"))

    def test_junit_names_the_outcome_and_keeps_both_attempts(self):
        h, _ = self.outcome("1")
        junit = os.path.join(h.report, "junit.xml")
        self.assertEqual(root_outcome(junit), "flaked")
        gate = suites(junit)["gate lane"]
        self.assertIn('failures="0"', gate)
        self.assertIn('<property name="outcome" value="flaked"/>', gate)
        self.assertEqual(gate.count("<properties>"), 1, "the reporter's own properties element takes the outcome property")
        self.assertIn('<property name="specId" value="0"/>', gate)
        self.assertNotIn("<failure", read(junit))
        for name in ("attempt-1-junit.xml", "attempt-2-junit.xml"):
            self.assertTrue(os.path.isfile(os.path.join(h.report, name)), name)
            # wdio.conf.ts onComplete merges and deletes every junit-<cid>.xml of the report directory.
            self.assertIsNone(re.match(r"^junit-.+\.xml$", name), name)
        self.assertEqual(read(os.path.join(h.report, "attempt-1-junit.xml")).count("<failure"), 1)

    def test_a_plain_red_junit_names_failed(self):
        h, _ = self.outcome("1,2")
        junit = os.path.join(h.report, "junit.xml")
        self.assertEqual(root_outcome(junit), "failed")
        self.assertEqual(read(junit).count("<failure"), 1)

    def test_a_failure_that_names_no_spec_is_red_without_a_rerun(self):
        h = Harness("1")
        self.addCleanup(h.close)
        stub = open(h.stub).read().replace('open(failed, "w").write("specs/gate.e2e.ts\\n")', "pass")
        open(h.stub, "w").write(stub)
        proc = h.run()
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(len(h.calls()), 1)
        self.assertEqual(h.record()["outcome"], "failed")
        self.assertTrue(os.path.isdir(os.path.join(h.report, "first-failure")))


class Wiring(unittest.TestCase):
    def text(self, *path):
        with open(os.path.join(REPO, *path)) as f:
            return f.read()

    def test_make_e2e_runs_the_suite_through_the_runner(self):
        target = re.search(r"^e2e:.*?(?=^clean:)", self.text("Makefile"), re.M | re.S).group(0)
        self.assertIn("scripts/e2e_run.py run", target)
        self.assertIn("wdio run wdio.conf.ts", target)

    def test_the_wdio_hooks_record_the_failed_spec_and_call_the_capture(self):
        conf = self.text("apps", "observatory-app", "e2e", "wdio.conf.ts")
        self.assertIn("failed-specs.txt", conf)
        self.assertIn("e2e_run.py", conf)
        self.assertIn("first-failure", conf)

    def test_an_env_knob_fails_one_spec_on_the_first_attempt_only(self):
        knob = self.text("apps", "observatory-app", "e2e", "lib", "induce.ts")
        self.assertIn("E2E_INDUCE_FAILURE", knob)
        self.assertIn("E2E_ATTEMPT", knob)
        self.assertIn("E2E_INDUCE_FAILURE", self.text("apps", "observatory-app", "e2e", "README.md"))

    def test_the_script_imports_only_the_standard_library(self):
        mods = set(re.findall(r"^(?:import|from)\s+([A-Za-z_]\w*)", self.text("scripts", "e2e_run.py"), re.M))
        allowed = {"argparse", "datetime", "glob", "json", "os", "re", "shutil", "subprocess", "sys", "time", "xml", "pathlib"}
        self.assertLessEqual(mods, allowed, mods - allowed)


if __name__ == "__main__":
    unittest.main()
