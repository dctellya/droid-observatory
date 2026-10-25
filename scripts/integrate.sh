#!/bin/sh
# Milestone boundary as a target (owner item 19, VAL-TOOL-006): `make integrate M=<milestone> FROM=<branch>...`.
#   1. refuse on a dirty tree (build-log/ may differ)
#   2. git merge --no-ff of each branch, message from scripts/templates/integrate-commit.txt, trailers from .git-trailers
#   3. final gate through `make check` (scripts/gate.sh): a hit when the merged tree is a proven tree, a real run on a miss;
#      red aborts with the summary, nothing is rerun
#   4. make usage M=<milestone>
#   5. merge reports/staging/*.md into SPEC.md section 9, BACKLOG.md and the decision files, delete them,
#      make decisions-index (scripts/integrate-staging.py)
#   6. write the boundary report reports/readiness/<date>-<milestone>.md with the gate numbers copied from the record
#   7. commit the boundary files and print the boundary summary
# Forward only: a failed step leaves the merges in place; fix with a new commit and run the target again (a branch that
# is already merged is skipped). INTEGRATE_DATE overrides the date (tests). CAPS and USAGE_ARGS reach `make usage`.
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

# (1) the tree must be clean: the gate key hashes the whole working tree, so stray files would turn a hit into a miss
dirty=$(git status --porcelain -- . ':(exclude)build-log')
if [ -n "$dirty" ]; then
  echo "integrate: working tree not clean (only build-log/ may differ):"
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
echo "integrate: final gate on tree $tree"
{ make --no-print-directory check; echo $? > "$tmp/gate.code"; } 2>&1 | tee "$tmp/gate.out"
gcode=$(cat "$tmp/gate.code" 2> /dev/null || echo 1)
summary=
[ -f "$rec" ] && summary=$(jq -r .summary "$rec")
[ -n "$summary" ] || summary=$(tail -n 5 "$tmp/gate.out")
if [ "$gcode" -ne 0 ]; then
  echo
  echo "gate: RED (make check exit $gcode) on tree $tree"
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
  gnote="the merged tree is a proven tree ($proven${validated:+; record present on$validated})"
  greport="cache hit, $proven"
else
  gstate=MISS
  reason=
  for b in $branches; do
    reason="$reason$(git diff --stat "$b" HEAD -- . ':(exclude)build-log' ':(exclude)reports/gate')"
  done
  [ -n "$reason" ] || reason="no check record for this tree on the branches (inner gates may have hit)"
  gnote="no record for the merged tree, the gate ran for real; reason: git diff --stat <branch> HEAD:
$reason"
  greport="real run, no record for the merged tree"
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
  echo "Tree \`$tree\`: $greport. Numbers below are copied from \`reports/gate/$tree/*.json\`."
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
echo "gate: $gstate on tree $tree"
echo "  $gnote"
echo "  summary: $(echo "$summary" | tr '\n' ' ')"
echo "usage: reports/usage/$m.md"
echo "staging:"
sed 's/^/  /' "$tmp/staging.summary"
echo "boundary report: $report"
echo "commits since start:"
git log --format='  %h %s' "$start..HEAD"
