#!/usr/bin/env python3
"""Runs the window suite (`make e2e`) with first-failure evidence and at most one rerun; Python 3 standard library only.

  e2e_run.py run --report-dir <dir> --run-dir <dir> [--cwd <dir>] -- <suite command...>
  e2e_run.py first-failure --report-dir <dir> --run-dir <dir> [--port <n>]...

`run` starts the suite command with E2E_ATTEMPT=1. When it fails, the evidence of that attempt is kept in
<report-dir>/first-failure/ before anything else runs; the failed specs (one path per line in <report-dir>/failed-specs.txt,
written by the wdio hooks) are rerun once, on a clean run directory, with `--spec <path>` appended to the command. The outcome
is `passed` (first attempt green), `flaked` (first attempt red, rerun green: the exit code is 0 and the record says so) or
`failed` (the rerun is red too, or no spec was named so nothing could be rerun). <report-dir>/record.json keeps both attempts
and the wall time of each, and junit.xml carries the outcome as an attribute of its root element
(owner inbox file docs/owner-feedback/inbox/2026-10-29-1552-flaky-e2e-is-a-feature.md).

`first-failure` is the capture alone, called by the wdio hooks while the app is still up so the ports and pids are live.
"""

import argparse
import datetime
import glob
import json
import os
import re
import shutil
import subprocess
import sys
import time

FIRST_FAILURE = "first-failure"
FAILED_SPECS = "failed-specs.txt"
# The embedded WebDriver server of wdio.conf.ts (embeddedPort).
DEFAULT_PORTS = (4445,)
PROCESS_PATTERN = re.compile(r"observatory-app|fake-droid|wdio|tauri-plugin", re.I)
MAX_PROCESS_LINES = 20


def now_utc():
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def command_lines(args):
    try:
        out = subprocess.run(args, capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.SubprocessError):
        return None
    return [line for line in out.stdout.splitlines() if line.strip()]


def run_dir_listing(run_dir):
    if not os.path.isdir(run_dir):
        return "run directory %s does not exist\n" % run_dir
    lines = ["%s\n" % run_dir]
    for root, dirs, files in os.walk(run_dir):
        dirs.sort()
        for name in sorted(dirs) + sorted(files):
            path = os.path.join(root, name)
            try:
                st = os.lstat(path)
            except OSError:
                continue
            kind = "d" if os.path.isdir(path) and not os.path.islink(path) else "-"
            stamp = datetime.datetime.fromtimestamp(st.st_mtime, datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
            lines.append("%s %10d %s %s\n" % (kind, st.st_size, stamp, os.path.relpath(path, run_dir)))
    return "".join(lines)


def environment(run_dir, report_dir, ports, env):
    """The facts a later reader needs about the run; the token is recorded as set or unset, never its value."""
    listening = {}
    for port in ports:
        lines = command_lines(["lsof", "-nP", "-iTCP:%d" % port, "-sTCP:LISTEN"])
        listening[str(port)] = "lsof unavailable" if lines is None else ("\n".join(lines) if lines else "not listening")
    processes = command_lines(["ps", "-axo", "pid,ppid,etime,command"]) or []
    return {
        "captured_at": now_utc(),
        "E2E_RUN_DIR": run_dir,
        "E2E_REPORT_DIR": report_dir,
        "E2E_DATE": env.get("E2E_DATE", "unset"),
        "E2E_ATTEMPT": env.get("E2E_ATTEMPT", "unset"),
        "E2E_INDUCE_FAILURE": env.get("E2E_INDUCE_FAILURE", "unset"),
        "GITHUB_TOKEN": "set" if env.get("GITHUB_TOKEN") else "unset",
        "app_github_token": "the harness value" if env.get("GITHUB_TOKEN") else "run-time fake (e2e/lib/env.ts)",
        "FORGE_LIVE": env.get("FORGE_LIVE", "unset"),
        "ports": listening,
        "pids": {
            "runner": env.get("E2E_RUNNER_PID"),
            "capture_parent": os.getppid(),
            "processes": [p.strip() for p in processes if PROCESS_PATTERN.search(p)][:MAX_PROCESS_LINES],
        },
    }


def copy_matching(pattern, dest):
    found = sorted(glob.glob(pattern))
    if found:
        os.makedirs(dest, exist_ok=True)
    for path in found:
        shutil.copy2(path, os.path.join(dest, os.path.basename(path)))


def capture_first_failure(report_dir, run_dir, ports=DEFAULT_PORTS, env=None, final=False):
    """Writes <report_dir>/first-failure/. The first call of the first attempt writes the environment and the run directory
    listing and later calls never replace them; every call of the first attempt refreshes the logs and screenshots; any call
    of a later attempt writes nothing, so the rerun cannot overwrite the evidence of the failure it reran."""
    env = os.environ if env is None else env
    if env.get("E2E_ATTEMPT", "1") != "1":
        return None
    folder = os.path.join(report_dir, FIRST_FAILURE)
    os.makedirs(folder, exist_ok=True)
    env_file = os.path.join(folder, "environment.json")
    if not os.path.exists(env_file):
        with open(env_file, "w") as f:
            json.dump(environment(run_dir, report_dir, ports, env), f, indent=2)
            f.write("\n")
    listing_file = os.path.join(folder, "run-dir-listing.txt")
    if not os.path.exists(listing_file):
        with open(listing_file, "w") as f:
            f.write(run_dir_listing(run_dir))
    if final:
        with open(os.path.join(folder, "run-dir-listing-at-end-of-attempt.txt"), "w") as f:
            f.write(run_dir_listing(run_dir))
    copy_matching(os.path.join(report_dir, "wdio-*.log"), os.path.join(folder, "backend-logs"))
    copy_matching(os.path.join(report_dir, "fail-*.png"), os.path.join(folder, "screenshots"))
    for name in (FAILED_SPECS, "timing.jsonl"):
        if os.path.exists(os.path.join(report_dir, name)):
            shutil.copy2(os.path.join(report_dir, name), os.path.join(folder, name))
    return folder


def read_failed_specs(report_dir):
    try:
        with open(os.path.join(report_dir, FAILED_SPECS)) as f:
            return sorted({line.strip() for line in f if line.strip()})
    except OSError:
        return []


def execute(cmd, cwd, env, attempt, report_dir, run_dir):
    child = dict(env, E2E_ATTEMPT=str(attempt), E2E_REPORT_DIR=report_dir, E2E_RUN_DIR=run_dir, E2E_RUNNER_PID=str(os.getpid()))
    started = now_utc()
    t0 = time.monotonic()
    rc = subprocess.run(cmd, cwd=cwd, env=child).returncode
    return {"attempt": attempt, "started_at": started, "seconds": round(time.monotonic() - t0, 2), "exit_code": rc,
            "outcome": "passed" if rc == 0 else "failed"}


SUITE = re.compile(r"<testsuite\b[^>]*?(?:/>|>.*?</testsuite>)", re.S)
OPEN_TAG = re.compile(r"<testsuite\b[^>]*?(/?)>")


def attribute(tag, name):
    found = re.search(r'\b%s="([^"]*)"' % name, tag)
    return found.group(1) if found else None


def failures_of(tag):
    return int(attribute(tag, "failures") or 0) + int(attribute(tag, "errors") or 0)


def write_junit(report_dir, outcome, rerun_path=None):
    """junit.xml = attempt 1 with each failed suite replaced by the rerun's suite of the same name (marked with the outcome
    property); the root element names the outcome. The attempts stay as junit-attempt-1.xml and junit-attempt-2.xml.
    Read as text: the junit reporter's output is flat (no nested suites) and the stdlib XML parser is not always loadable
    (Homebrew's python 3.14.7 here fails to import pyexpat)."""
    try:
        with open(os.path.join(report_dir, "junit-attempt-1.xml")) as f:
            text = f.read()
    except OSError:
        return
    rerun = {}
    if rerun_path and os.path.exists(rerun_path):
        with open(rerun_path) as f:
            rerun = {attribute(OPEN_TAG.match(s).group(0), "name"): s for s in SUITE.findall(f.read())}

    def replace(match):
        suite = match.group(0)
        opening = OPEN_TAG.match(suite)
        again = rerun.get(attribute(opening.group(0), "name"))
        if failures_of(opening.group(0)) == 0 or again is None:
            return suite
        props = ('<properties><property name="outcome" value="%s"/><property name="first_attempt_failures" value="%d"/>'
                 "</properties>" % (outcome, failures_of(opening.group(0))))
        head = OPEN_TAG.match(again)
        return again if head.group(1) else again[:head.end()] + props + again[head.end():]

    text = SUITE.sub(replace, text)
    text = re.sub(r'(<testsuites\b[^>]*?)(\s+outcome="[^"]*")?(\s*/?>)', lambda m: '%s outcome="%s"%s' % (m.group(1), outcome, m.group(3)),
                  text, count=1)
    with open(os.path.join(report_dir, "junit.xml"), "w") as f:
        f.write(text)


def run(report_dir, run_dir, cmd, cwd=None, env=None):
    env = dict(os.environ if env is None else env)
    shown = report_dir
    report_dir, run_dir = os.path.abspath(report_dir), os.path.abspath(run_dir)
    os.makedirs(report_dir, exist_ok=True)
    started = now_utc()
    first = execute(cmd, cwd, env, 1, report_dir, run_dir)
    attempts = [first]
    record = {"run": os.path.basename(report_dir), "started_at": started}
    rerun_path = None
    if first["exit_code"] == 0:
        record["outcome"] = "passed"
    else:
        failed = read_failed_specs(report_dir)
        first["failed_specs"] = failed
        capture_first_failure(report_dir, run_dir, env=dict(env, E2E_ATTEMPT="1"), final=True)
        record["first_failure"] = os.path.join(shown, FIRST_FAILURE)
        for name, kept in (("junit.xml", "junit-attempt-1.xml"), (FAILED_SPECS, "failed-specs-attempt-1.txt")):
            if os.path.exists(os.path.join(report_dir, name)):
                shutil.move(os.path.join(report_dir, name), os.path.join(report_dir, kept))
        if not failed:
            record["outcome"] = "failed"
            record["failed_specs"] = []
        else:
            shutil.rmtree(run_dir, ignore_errors=True)
            second = execute(cmd + [a for spec in failed for a in ("--spec", spec)], cwd, env, 2, report_dir, run_dir)
            second["specs"] = failed
            attempts.append(second)
            record["rerun_seconds"] = second["seconds"]
            if os.path.exists(os.path.join(report_dir, "junit.xml")):
                shutil.move(os.path.join(report_dir, "junit.xml"), os.path.join(report_dir, "junit-attempt-2.xml"))
                rerun_path = os.path.join(report_dir, "junit-attempt-2.xml")
            if second["exit_code"] == 0:
                record["outcome"] = "flaked"
                record["flaked_specs"] = failed
            else:
                record["outcome"] = "failed"
                second["failed_specs"] = read_failed_specs(report_dir) or failed
                record["failed_specs"] = second["failed_specs"]
    record["attempts"] = attempts
    record["finished_at"] = now_utc()
    if record["outcome"] == "passed":
        junit = os.path.join(report_dir, "junit.xml")
        if os.path.exists(junit):
            os.replace(junit, os.path.join(report_dir, "junit-attempt-1.xml"))
    write_junit(report_dir, record["outcome"], rerun_path)
    with open(os.path.join(report_dir, "record.json"), "w") as f:
        json.dump(record, f, indent=2)
        f.write("\n")
    print(summary_line(record))
    return 0 if record["outcome"] in ("passed", "flaked") else (attempts[-1]["exit_code"] or 1)


def summary_line(record):
    attempts = record["attempts"]
    total = sum(a["seconds"] for a in attempts)
    if record["outcome"] == "passed":
        return "e2e: passed (%.2f s)" % total
    if record["outcome"] == "flaked":
        return "e2e: flaked (first attempt red, rerun green): %s; rerun %.2f s of %.2f s; evidence %s" % (
            ", ".join(record["flaked_specs"]), record["rerun_seconds"], total, record["first_failure"])
    return "e2e: failed (%s): %s; evidence %s" % (
        "red again on the rerun" if len(attempts) > 1 else "no failed spec named, no rerun",
        ", ".join(record["failed_specs"]) or "unknown", record["first_failure"])


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="command", required=True)
    r = sub.add_parser("run", help="run the suite command with the rerun policy")
    r.add_argument("--report-dir", required=True)
    r.add_argument("--run-dir", required=True)
    r.add_argument("--cwd", help="directory the suite command runs in")
    r.add_argument("cmd", nargs=argparse.REMAINDER, help="-- <suite command...>")
    c = sub.add_parser("first-failure", help="capture the evidence of the first failing spec")
    c.add_argument("--report-dir", required=True)
    c.add_argument("--run-dir", required=True)
    c.add_argument("--port", type=int, action="append", help="a port to report on (default 4445)")
    args = ap.parse_args(argv)
    if args.command == "first-failure":
        capture_first_failure(os.path.abspath(args.report_dir), os.path.abspath(args.run_dir), tuple(args.port or DEFAULT_PORTS))
        return 0
    cmd = args.cmd[1:] if args.cmd[:1] == ["--"] else args.cmd
    if not cmd:
        ap.error("no suite command after --")
    return run(args.report_dir, args.run_dir, cmd, cwd=args.cwd)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
