# D-021: The tool-call hook logs a `script` field

**Date:** 2026-10-23
**Status:** accepted (owner decision 2026-10-23 at the S to M0 boundary); addendum 2026-10-23 (orchestrator, rule 24, D-025 duty 5): the hook also runs on PreToolUse and appends `call_id` (hash of tool name and input; the payload has no tool_use_id), `ts_ms` (UTC milliseconds) and, on PostToolUse, `duration_ms` (end minus the matching start, `null` without one); the nine fields above are unchanged, `scripts/compile-share.sh` sums them

## Context

`.factory/hooks/log-tool-call.sh` (rule 17) records `command`, `file` and `summary` for each tool
call. A `Script` tool call carries its code in `tool_input.script`, which none of those fields
holds, so the log showed that a Script ran but not what it did. Scrutiny at S could not use the
build-log as red-test evidence for work done through Script calls.

## Decision

The hook adds `script`: the first 200 characters of `tool_input.script`, or `null` when the tool
input has no `script` (any other tool, or a string or array `tool_input`):

```jq
script: (.tool_input.script? // null | if . == null then null else .[0:200] end)
```

Checked on 2026-10-23: a Script payload with a 300-character script logs a `script` of length
200 (`jq -r '.script|length'` prints `200`); an `Execute` payload logs `"script":null`; a payload
whose `tool_input` is a string still writes a line with `"script":null`.

## Alternatives

Log the whole script (rejected: unbounded size, and scripts can embed secrets or file contents
that the log was designed to leave out); log a hash only (rejected: not readable as evidence);
leave it (rejected: Script calls stay invisible).

## Consequences

The build-log grows by at most 200 characters per Script call. The hook still never records the
tool response. Scripts longer than 200 characters are visible only as a prefix; the session
transcript holds the rest. The hook file change is allowed by this decision (the mission
otherwise keeps hooks unchanged).
