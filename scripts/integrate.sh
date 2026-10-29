#!/bin/sh
# Milestone boundary as a target (owner item 19, VAL-TOOL-006): `make integrate M=<milestone> FROM=<branch>...`.
#   1. refuse on a dirty tree (build-log/ may differ)
#   1b. validate every staging file of the branches still to merge (stray `##` headings, file:line, exit 2) before any merge
#   2. git merge --no-ff of each branch, message from scripts/templates/integrate-commit.txt, trailers from .git-trailers
#   3. final gate through `make check` (scripts/gate.sh), keyed by the code paths: a HIT when the merged code paths equal a
#      proven gate, a real run on a MISS that lists the code files that differ; red aborts with the summary, nothing is rerun
#   3b. the long work (scripts/integrate-long.sh, `make integrate-long`): its result for the merged code tree is read from
#      reports/integrate-long/<code tree>.json into the report and the summary; INTEGRATE_LONG=1 runs it here first (the
#      branch must be pushed, the ci job waits for the pushed head). A pending or red result does not stop the boundary: a
#      red one fails the target after the boundary commit; after a separate `make integrate-long`, run this target again
#      (merged branches are skipped, the gate is a hit) to refresh the report
#   4. make usage M=<milestone>
#   5. merge reports/staging/*.md into SPEC.md section 9, BACKLOG.md and the decision files, delete them,
#      make decisions-index (scripts/integrate-staging.py)
#   6. write the boundary report reports/readiness/<date>-<milestone>.md with the gate numbers copied from the record
#   7. commit the boundary files and print the boundary summary
# Forward only: a failed step leaves the merges in place; fix with a new commit and run the target again (a branch that
# is already merged is skipped). INTEGRATE_DATE overrides the date (tests). CAPS and USAGE_ARGS reach `make usage`.
# shellcheck disable=SC2016 # backticks in single quotes are Markdown, not command substitution
set -u
LC_ALL=C
export LC_ALL

if [ $# -lt 2 ]; then
  echo "usage: integrate.sh <milestone> <branch>...   (make integrate M=<milestone> FROM='<branch>...')" >&2
  exit 2
fi
m=$1
shift
here=$(cd "$(dirname "$0")" && pwd)
cd "$(git rev-parse --show-toplevel)" || exit 2
command -v jq > /dev/null 2>&1 || { echo "integrate: jq is required" >&2; exit 2; }
[ -f .git-trailers ] || { echo "integrate: .git-trailers is missing" >&2; exit 2; }
date=${INTEGRATE_DATE:-$(date -u +%Y-%m-%d)}
branches="$*"

tmp=$(mktemp -d "${TMPDIR:-/tmp}/integrate.XXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM

# (1) the tree must be clean: the gate key hashes the code paths of the working tree, so stray code files would turn a hit into a miss.
# reports/integrate-long/ is output of the long work, written between two runs of this target and committed by it.
dirty=$(git status --porcelain -- . ':(exclude)build-log' ':(exclude)reports/integrate-long')
if [ -n "$dirty" ]; then
  echo "integrate: working tree not clean (only build-log/ and reports/integrate-long/ may differ):"
  echo "$dirty"
  exit 1
fi
mission=$(git rev-parse --abbrev-ref HEAD)
for b in $branches; do
  git rev-parse --verify -q "$b^{commit}" > /dev/null || { echo "integrate: unknown branch $b"; exit 1; }
  [ "$b" != "$mission" ] || { echo "integrate: $b is the current branch"; exit 1; }
done
start=$(git rev-parse HEAD)

# Fill {{placeholders}} of a template; awk reads the values from the environment so no character needs escaping.
render() { # render <template> <branch> <commits-file>
  T_MILESTONE=$m T_BRANCH=$2 T_DATE=$date awk -v commits="$3" '
    function rep(s, key, val,   out, i) {
      out = ""
      while ((i = index(s, key)) > 0) { out = out substr(s, 1, i - 1) val; s = substr(s, i + length(key)) }
      return out s
    }
    $0 == "{{commits}}" { while ((getline line < commits) > 0) print line; next }
    { $0 = rep($0, "{{milestone}}", ENVIRON["T_MILESTONE"]); $0 = rep($0, "{{branch}}", ENVIRON["T_BRANCH"])
      $0 = rep($0, "{{date}}", ENVIRON["T_DATE"]); print }' "$1"
}
add_trailers() { # add_trailers <message-file>: the trailers file is the source, never a heredoc tail
  while IFS= read -r t || [ -n "$t" ]; do
    [ -z "$t" ] || git interpret-trailers --in-place --if-exists addIfDifferent --trailer "$t" "$1"
  done < .git-trailers
}

# (1b) every staging file of every branch still to merge is validated before the first merge, all problems in one pass
bad=0
for b in $branches; do
  git merge-base --is-ancestor "$b" HEAD && continue
  python3 "$here/integrate-staging.py" --check --rev "$b" || bad=1
done
if [ "$bad" -ne 0 ]; then
  echo "integrate: staging files are invalid; fix them on the branch (new commit) and run the target again. Nothing was merged."
  exit 2
fi

# (2) one no-ff merge per branch
for b in $branches; do
  if git merge-base --is-ancestor "$b" HEAD; then
    echo "integrate: $b already merged into $mission, skipping the merge"
    continue
  fi
  git log --format='- %s' "HEAD..$b" > "$tmp/commits"
  render "$here/templates/integrate-commit.txt" "$b" "$tmp/commits" > "$tmp/merge.msg"
  add_trailers "$tmp/merge.msg"
  if ! git merge --no-ff --no-edit -F "$tmp/merge.msg" "$b"; then
    echo "integrate: merge of $b failed; resolve with a new commit (forward only) and run the target again"
    exit 1
  fi
  echo "integrate: merged $b as $(git log -1 --format='%h %s')"
done

# (3) the final gate
tree=$(sh scripts/gate.sh --key | sed -n 1p)
rec="reports/gate/$tree/check.json"
echo "integrate: final gate on code tree $tree"
{ make --no-print-directory check; echo $? > "$tmp/gate.code"; } 2>&1 | tee "$tmp/gate.out"
gcode=$(cat "$tmp/gate.code" 2> /dev/null || echo 1)
summary=
[ -f "$rec" ] && summary=$(jq -r .summary "$rec")
[ -n "$summary" ] || summary=$(tail -n 5 "$tmp/gate.out")
if [ "$gcode" -ne 0 ]; then
  echo
  echo "gate: RED (make check exit $gcode) on code tree $tree"
  echo "$summary"
  echo "integrate: aborted before usage and staging; the merges stay. Fix with a new commit and run the target again."
  exit 1
fi
# Only the check gate's own first line decides: the inner lint and test gates print "cached, proven at" too.
first=$(grep -m 1 -E '^(cached, proven at |gate check: (miss|forced) )' "$tmp/gate.out")
case "$first" in "cached, proven at "*) check_hit=1 ;; *) check_hit=0 ;; esac
if [ "$check_hit" -eq 1 ]; then
  gstate=HIT
  proven=$(jq -r '"proven at \(.finished_at) by \(.session)"' "$rec")
  validated=
  for b in $branches; do
    git cat-file -e "$b:$rec" 2> /dev/null && validated="$validated $b"
  done
  gnote="the merged code tree is a proven tree ($proven${validated:+; record present on$validated})"
  greport="cache hit, $proven"
else
  gstate=MISS
  reason=
  for b in $branches; do
    reason="$reason$(sh scripts/gate.sh --diff "$b")"
  done
  [ -n "$reason" ] || reason="no check record for this code tree on the branches (inner gates may have hit)"
  gnote="no record for the merged code tree, the gate ran for real; reason: code files that differ (gate.sh --diff <branch>):
$reason"
  greport="real run, no record for the merged code tree"
fi

# (3b) the long work
lrec="reports/integrate-long/$tree.json"
if [ "${INTEGRATE_LONG:-}" = 1 ]; then
  echo "integrate: integrate-long on code tree $tree (the ci job needs the branch pushed)"
  sh "$here/integrate-long.sh" "$m" || echo "integrate: the long work is red; the boundary continues and the target fails at the end"
fi
lstate=pending
if [ -f "$lrec" ]; then
  lworst=$(jq -r .exit_code "$lrec")
  if [ "$lworst" = 0 ]; then lstate=OK; else lstate=RED; fi
  lnote="long work: $lstate (worst exit code $lworst) on code tree $tree, $lrec"
else
  lnote="long work: not run for code tree $tree; push the branch, start 'make integrate-long M=$m' as a background job, then run 'make integrate M=$m FROM=\"$branches\"' again to refresh the report"
fi

# (4) usage
echo "integrate: make usage M=$m"
make --no-print-directory usage M="$m" || { echo "integrate: make usage failed; fix and run the target again"; exit 1; }

# (5) staging files into the shared files
mkdir -p reports/staging reports/readiness
python3 "$here/integrate-staging.py" --milestone "$m" --date "$date" --out "$tmp/staging.md" > "$tmp/staging.summary" \
  || { echo "integrate: staging merge failed"; exit 1; }
make --no-print-directory decisions-index

# (6) the boundary report; numbers come from the gate records, nothing is measured again
report="reports/readiness/$date-$m.md"
{
  echo "<!-- integrate:begin -->"
  echo
  echo "Written by \`make integrate\` on $date. Branches: $branches. Mission branch: \`$mission\`."
  echo
  echo "## Merges"
  echo
  git log --merges --format='- `%h` parents `%p`: %s' "$start..HEAD"
  echo
  echo "## Final gate"
  echo
  echo "Code tree \`$tree\`: $greport. Numbers below are copied from \`reports/gate/$tree/*.json\`."
  echo
  echo "| Gate | Exit | Seconds | Proven at | Session |"
  echo "|---|---|---|---|---|"
  for f in "reports/gate/$tree"/*.json; do
    [ -f "$f" ] && jq -r '"| \(.name) | \(.exit_code) | \(.seconds) | \(.finished_at) | \(.session) |"' "$f"
  done
  for f in "reports/gate/$tree"/*.json; do
    [ -f "$f" ] || continue
    echo
    echo "Summary of \`$(jq -r .name "$f")\`:"
    echo
    echo '```text'
    jq -r .summary "$f"
    echo '```'
  done
  echo
  echo "## Long work"
  echo
  if [ -f "$lrec" ]; then
    echo "Code tree \`$tree\`: $lstate, worst exit code $(jq -r .exit_code "$lrec"), $(jq -r .seconds "$lrec") s wall, finished $(jq -r .finished_at "$lrec") (\`$lrec\`)."
    echo
    echo "| Job | Lane | Exit | Seconds | Reused | Log |"
    echo "|---|---|---|---|---|---|"
    jq -r '.jobs[] | "| \(.name) | \(.lane) | \(.exit_code) | \(.seconds) | \(.hit) | \(.log) |"' "$lrec"
    for n in $(jq -r '.jobs[].name' "$lrec"); do
      g="reports/gate/$tree/$n.json"
      [ -f "$g" ] || continue
      echo
      echo "Summary of \`$n\`:"
      echo
      echo '```text'
      jq -r .summary "$g"
      echo '```'
    done
  else
    echo "The long work is pending: no result for code tree \`$tree\`. After the push, start \`make integrate-long M=$m\` as a background job and run \`make integrate\` again."
  fi
  echo
  echo "## Staging"
  echo
  cat "$tmp/staging.md"
  echo
  echo "<!-- integrate:end -->"
} > "$tmp/report.block"
if [ -f "$report" ] && grep -q '^<!-- integrate:begin -->' "$report"; then
  awk -v block="$tmp/report.block" '
    /^<!-- integrate:begin -->/ { while ((getline l < block) > 0) print l; skip = 1; next }
    /^<!-- integrate:end -->/ { skip = 0; next }
    !skip { print }' "$report" > "$tmp/report.new" && cp "$tmp/report.new" "$report"
elif [ -f "$report" ]; then
  { cat "$report"; echo; cat "$tmp/report.block"; } > "$tmp/report.new" && cp "$tmp/report.new" "$report"
else
  { printf '# Integration report: %s\n\n' "$m"; cat "$tmp/report.block"; } > "$report"
fi

# (7) the boundary commit and the summary
git add -A -- . ':(exclude)build-log'
if git diff --cached --quiet; then
  echo "integrate: nothing to commit for the boundary files"
else
  render "$here/templates/integrate-boundary.txt" "$branches" /dev/null > "$tmp/boundary.msg"
  add_trailers "$tmp/boundary.msg"
  git commit -q -F "$tmp/boundary.msg" || { echo "integrate: boundary commit failed"; exit 1; }
fi

echo
echo "==== Boundary summary: $m (FROM $branches) ===="
echo "mission branch: $mission, start $(git rev-parse --short "$start"), end $(git log -1 --format='%h %s')"
echo "merges:"
git log --merges --format='  %h parents %p  %s' "$start..HEAD"
echo "gate: $gstate on code tree $tree"
echo "  $gnote"
echo "  summary: $(echo "$summary" | tr '\n' ' ')"
echo "  $lnote"
echo "usage: reports/usage/$m.md"
echo "staging:"
sed 's/^/  /' "$tmp/staging.summary"
echo "boundary report: $report"
echo "commits since start:"
git log --format='  %h %s' "$start..HEAD"
[ "$lstate" != RED ] || { echo "integrate: the long work is red (see $lrec)"; exit 1; }
