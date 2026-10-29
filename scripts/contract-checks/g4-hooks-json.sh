#!/bin/sh
# VAL-PROC-022 (hooks.json clause): guard-git.sh runs before tool calls and log-tool-call.sh after them.
. "$(dirname "$0")/lib.sh"
ids="VAL-PROC-022"
need_tool jq
[ -f .factory/hooks.json ] || { fail ".factory/hooks.json is missing"; finish "$ids"; }
jq -r '.PreToolUse[].hooks[].command' .factory/hooks.json 2> /dev/null | grep -q 'guard-git.sh' || fail "no PreToolUse hook names guard-git.sh"
jq -r '.PostToolUse[].hooks[].command' .factory/hooks.json 2> /dev/null | grep -q 'log-tool-call.sh' || fail "no PostToolUse hook names log-tool-call.sh"
finish "$ids"
