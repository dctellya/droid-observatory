"""A scratch copy of scripts/tests/fixtures/usage/ that a test can extend with its own log lines, progress records,
sessions and transcripts on a day (2026-10-26) the shared fixture does not use, so the shared fixture's numbers stay
the arithmetic of usage_test.py.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile

from usage_test import FIX, MISSION, SCRIPT, table  # noqa: F401  (table is re-exported for the tests)

DAY = "2026-10-26"
FROM = DAY + "T00:00:00Z"
TO = DAY + "T06:00:00Z"
NOW = "2026-10-29T10:34:11Z"
FABLE, OPUS, SONNET = "claude-fable-5.1", "claude-opus-5-5", "claude-sonnet-5-5"


def at(hhmmss):
    """A log timestamp on the scratch day: at("00:05:30") is 2026-10-26T00:05:30.000Z."""
    return "%sT%s.000Z" % (DAY, hhmmss)


class Scratch:
    def __init__(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = os.path.join(self.tmp.name, "usage")
        shutil.copytree(FIX, self.root)
        self.log = os.path.join(self.root, "logs", "droid-log-single.log")
        self.sessions = os.path.join(self.root, "sessions", "-home-user-proj")
        self.mission = os.path.join(self.root, "mission")

    def close(self):
        self.tmp.cleanup()

    def log_line(self, ts, text, ctx):
        with open(self.log, "a") as f:
            f.write("[%s] %s: %s | Context: %s\n" % (ts, "INFO", text, json.dumps(ctx)))

    def turn(self, ts, sid, model, i=0, o=0, cr=0, cw=0):
        self.log_line(ts, "[Agent] Streaming result", {
            "count": 1, "inputTokens": i, "cacheReadInputTokens": cr, "totalInputTokens": i + cr + cw, "reasoningTokens": 0,
            "outputTokens": o, "hasReasoningContent": False, "cachedTokensWritten": cw,
            "tags": {"clientType": "cli", "sessionId": sid, "modelId": model}})

    def raw_log(self, ts, level, text, ctx):
        with open(self.log, "a") as f:
            f.write("[%s] %s: %s | Context: %s\n" % (ts, level, text, json.dumps(ctx)))

    def progress(self, ts, kind, **fields):
        with open(os.path.join(self.mission, "progress_log.jsonl"), "a") as f:
            f.write(json.dumps(dict(timestamp=ts, type=kind, **fields)) + "\n")

    def sidecar(self, sid, model, tags):
        with open(os.path.join(self.sessions, sid + ".settings.json"), "w") as f:
            json.dump({"model": model, "tags": tags, "tokenUsage": None}, f)

    def worker_sidecar(self, sid, model=OPUS):
        self.sidecar(sid, model, [{"name": "mission-session", "metadata": {"role": "worker", "missionId": MISSION}}])

    def subagent_sidecar(self, sid, caller, tool_use, model=OPUS):
        self.sidecar(sid, model, [{"name": "subagent", "metadata": {"callingSessionId": caller, "callingToolUseId": tool_use}}])

    def transcript(self, sid, records):
        path = os.path.join(self.sessions, sid + ".jsonl")
        with open(path, "a") as f:
            for r in records:
                f.write(json.dumps(dict(type="message", **r)) + "\n")

    def features(self, features):
        path = os.path.join(self.mission, "features.json")
        with open(path) as f:
            data = json.load(f)
        data["features"] += features
        with open(path, "w") as f:
            json.dump(data, f)

    def run(self, extra=(), milestone="m9", start=FROM, end=TO, check=True):
        out = os.path.join(self.tmp.name, milestone + ".md")
        args = [sys.executable, SCRIPT, "--milestone", milestone, "--mission-dir", self.mission, "--mission-id", MISSION,
                "--logs-dir", os.path.join(self.root, "logs"), "--sessions-dir", os.path.join(self.root, "sessions"),
                "--prices", os.path.join(self.root, "prices.toml"), "--gate-dir", os.path.join(self.root, "gate"),
                "--from", start, "--to", end, "--now", NOW, "--out", out] + list(extra)
        proc = subprocess.run(args, capture_output=True, text=True)
        if check and proc.returncode != 0:
            raise AssertionError("usage-report.py exit %d\nstdout: %s\nstderr: %s" % (proc.returncode, proc.stdout, proc.stderr))
        proc.text = open(out).read() if os.path.exists(out) else ""
        return proc
