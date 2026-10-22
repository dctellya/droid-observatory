#!/bin/sh
# Factory PreToolUse hook: the write guard (ai/RULES.md rules 12 and 13).
# Blocks history rewrites and direct pushes to main from any agent. Exit 2 = block.
set -u
input=$(cat)
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')
[ -n "$cmd" ] || exit 0

deny() { echo "blocked by .factory/hooks/guard-git.sh: $1 (ai/RULES.md rule 12: git forward only)" >&2; exit 2; }

printf '%s' "$cmd" | grep -Eq 'git[[:space:]]+(commit[^|;&]*--amend|reset[[:space:]]+--(hard|soft|mixed)|rebase|stash|push[^|;&]*(--force|-f\b|\+))' && deny "history rewrite"
printf '%s' "$cmd" | grep -Eq 'git[[:space:]]+push[^|;&]*[[:space:]](origin[[:space:]]+)?(HEAD:)?main\b' && deny "direct push to main"
printf '%s' "$cmd" | grep -Eq 'git[[:space:]]+(checkout|switch)[[:space:]]+main\b' && printf '%s' "$cmd" | grep -Eq 'git[[:space:]]+commit' && deny "committing on main"
exit 0
