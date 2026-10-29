#!/bin/sh
# Factory PreToolUse hook: rule C, the officer's tools are an allow-list (owner inbox 1845 and 2049, 2026-10-29;
# D-041). For the sessions listed in officer-sessions.txt (the mission orchestrator) only the tools of the officer's
# activities pass; Read and LS are limited to the mission directory, the owner inbox and reports/staging of the worktree.
# Rule F (owner inbox 2247, D-042): a Script with startInBackground that calls StartMissionRun, DismissHandoffItems,
# EndFeatureRun or Task is denied; the rule E heartbeat (a background Script) passes.
# Every other session is untouched. Exit 2 = block.
set -u
input=$(cat)
session=$(printf '%s' "$input" | jq -r '.session_id // empty')
tool=$(printf '%s' "$input" | jq -r '.tool_name // empty')
[ -n "$session" ] && [ -n "$tool" ] || exit 0

list="$(dirname "$0")/officer-sessions.txt"
[ -f "$list" ] || exit 0
grep -Fxq "$session" "$list" || exit 0

deny_scope() { echo "outside the officer's read scope (rule C): ${1:-(no path)}" >&2; exit 2; }

in_scope() {
  # A `..` segment would walk out of a prefix that matched, so it never passes.
  case "$1" in */../* | */..) return 1 ;; esac
  case "$1" in
    /Users/dct/.factory/missions/e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c/* | \
      /Users/dct/code/EllyaOrg/projects/dalimaray/observatory-wt/m4/docs/owner-feedback/inbox* | \
      /Users/dct/code/EllyaOrg/projects/dalimaray/observatory-wt/m4/reports/staging*) return 0 ;;
  esac
  return 1
}

deny_background_decision() {
  # A decision call wrapped in a background Script returns out of order and unrecognised (rule F, D-042); the harness
  # exposes the runner tools only as Script bindings, so only the startInBackground form is refused.
  script=$(printf '%s' "$input" | jq -r '.tool_input.script? // empty')
  if printf '%s' "$script" | grep -Eq 'startInBackground' &&
    printf '%s' "$script" | grep -Eq 'tools\.(StartMissionRun|DismissHandoffItems|EndFeatureRun|Task)\('; then
    echo "decisions are direct calls (rule F)" >&2
    exit 2
  fi
}

case "$tool" in
  Script)
    deny_background_decision
    exit 0
    ;;
  Task | DismissHandoffItems | StartMissionRun | EndFeatureRun) exit 0 ;;
  Read)
    path=$(printf '%s' "$input" | jq -r '.tool_input.file_path? // empty')
    in_scope "$path" && exit 0
    deny_scope "$path"
    ;;
  LS)
    path=$(printf '%s' "$input" | jq -r '.tool_input.directory_path? // empty')
    in_scope "$path" && exit 0
    deny_scope "$path"
    ;;
  *)
    echo "not an officer tool (rule C): $tool" >&2
    exit 2
    ;;
esac
