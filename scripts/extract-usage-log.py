#!/usr/bin/env python3
"""Extracts the usage-log fixture of a capture from Droid's CLI log (F-034, D-029).

    scripts/extract-usage-log.py <capture.json> <out.usage-log.jsonl> <log>...

<log...> are given oldest first (droid-log-single.log.<date> before droid-log-single.log).
Keeps only the "[Agent] Streaming result" lines whose joined session id is one of the capture's
`.sessions[].id`, each preceded by the line it was joined to. The join is the one of
`observatory_core::ingest::usage_log`: the nearest preceding line whose Context JSON has a
top-level `sessionId`, if it carries the same second as the turn. Turns after the capture's
`captured_at` are left out: the session files stop there, so later turns would not reconcile
with them.

The kept lines are written as they are, except that the three machine identifiers of the tags
(`droidInstallationId`, `hostId`, `factory.process.instance_id`) are replaced by a same-length
fixed UUID. Run scripts/redact-fixture.sh on the output afterwards (emails, home paths).
The first line of stdout-style coverage goes to stderr.
"""
import json
import re
import sys

CONTEXT = " | Context: "
MACHINE_IDS = re.compile(
    r'("(?:droidInstallationId|hostId|factory\.process\.instance_id)":")'
    r"[0-9a-fA-F-]{36}(\")"
)
ZERO_UUID = "00000000-0000-4000-8000-000000000000"


def context_of(line):
    at = line.find(CONTEXT)
    if at < 0:
        return None
    try:
        value = json.loads(line[at + len(CONTEXT):])
    except ValueError:
        return None
    return value if isinstance(value, dict) else None


def main(capture, out, logs):
    with open(capture, encoding="utf-8") as handle:
        header = json.load(handle)
    wanted = {session["id"] for session in header["sessions"]}
    until = header["captured_at"][:19]
    neighbour = None  # (second, session id, line)
    emitted = None  # the neighbour line object last written
    kept = turns = unjoined = 0
    first = last = None
    with open(out, "w", encoding="utf-8", newline="\n") as sink:
        for path in logs:
            with open(path, encoding="utf-8", errors="replace") as source:
                for raw in source:
                    line = raw.rstrip("\n")
                    if not line.startswith("[") or len(line) < 26:
                        continue
                    second = line[1:20]
                    if "[Agent] Streaming result" in line[26:60]:
                        turns += 1
                        if neighbour is None or neighbour[0] != second:
                            unjoined += 1
                            continue
                        if neighbour[1] not in wanted or second > until:
                            continue
                        if emitted is not neighbour:
                            sink.write(MACHINE_IDS.sub(rf"\g<1>{ZERO_UUID}\g<2>", neighbour[2]) + "\n")
                            emitted = neighbour
                        sink.write(MACHINE_IDS.sub(rf"\g<1>{ZERO_UUID}\g<2>", line) + "\n")
                        kept += 1
                        first = first or line[1:25]
                        last = line[1:25]
                    elif '"sessionId":"' in line:
                        context = context_of(line)
                        if context and isinstance(context.get("sessionId"), str):
                            neighbour = (second, context["sessionId"], line)
    print(
        f"streaming_result_lines={turns} kept={kept} unjoined={unjoined} "
        f"first_ts={first} last_ts={last}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    if len(sys.argv) < 4:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2], sys.argv[3:])
