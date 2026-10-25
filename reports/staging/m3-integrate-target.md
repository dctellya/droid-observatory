# Staging: m3-integrate-target

Feature `m3-integrate-target` (owner M2-boundary item 19), worktree `observatory-wt/m3`, branch `wt/m3`.
VAL-TOOL-006 (proven at `m4-integrate-m3`). Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

- Row (F-NEW): `make integrate M=<milestone> FROM=<branch>...` is `scripts/integrate.sh`. It refuses on a dirty tree
  (only `build-log/` may differ; stray files would turn a gate hit into a miss), merges each branch with
  `git merge --no-ff --no-edit -F` of a message rendered from `scripts/templates/integrate-commit.txt` (subject
  `Integrate <m>: <branch>`, the merged subjects as the body) whose trailers are added by
  `git interpret-trailers --in-place --if-exists addIfDifferent` from `.git-trailers`, runs the final gate as
  `make check` (so `scripts/gate.sh` prints `cached, proven at <time> by <session>` when the merged tree is a proven
  tree and runs for real on a miss, with `git diff --stat <branch> HEAD` as the reason), runs `make usage M=<m>`,
  merges the staging files, regenerates the decisions index and commits the boundary files with
  `scripts/templates/integrate-boundary.txt` (`chore(integration): <m> boundary reports and shared files`).
  A red gate prints `gate: RED` with the stored summary and exits 1 without a rerun; the merges stay (forward
  only) and a second run skips branches that are already merged, so the fix is a new commit and the same command.
  Alternatives considered: `git commit --trailer` (no such option on `git merge`); `commit.template` (git-commit: no
  effect with `-F`); the research's `sed` template rendering (replaced by `awk` so a branch name with `/` or `&` needs
  no escaping).
- Row (F-NEW): staging merge rules, `scripts/integrate-staging.py` (python3 stdlib). Files are read in name order. A
  bullet of `## SPEC §9` becomes the next `F-NNN` row of the SPEC §9 table (`Applies` = the feature, `Status` =
  `done (<m> integration)`, `|` escaped). A `## Decisions` bullet `D-NNN (amend...): "text"` becomes a section
  `## Addendum <date> (<m> integration, <feature>)` at the end of `docs/decisions/D-NNN-*.md`; other bullets are only
  listed in the report. A `## BACKLOG` bullet starting `B-NEW` becomes the next `B-NNN` row (item and reason split at
  `reason:`, otherwise at the first sentence; `Earliest` = `unscheduled`, the feature id appended to the reason); other
  bullets (for example `Closed (do not file)`) are only listed. `## Deferred clauses` and `## Library` are copied per
  feature into `reports/readiness/<date>-<m>.md`, between `<!-- integrate:begin -->` and `<!-- integrate:end -->`
  (prose outside the markers is kept). Run on the six staging files now in this worktree (copies, nothing written): F-071
  to F-083, B-078 to B-088 and three addenda (D-029 twice, D-033) came out as expected.
- Row (F-NEW): the boundary report copies the gate numbers from `reports/gate/<tree>/*.json` (name, exit code,
  seconds, finished_at, session, summary); nothing is measured again.

## Decisions

None.

## BACKLOG

- B-NEW: the staging merge assigns `Earliest` = `unscheduled` to every B-row and does not read a milestone out of the
  bullet. Reason: the milestone is a judgement the integrator makes; add a `[Mx]` tag convention if it should be
  automatic.
- B-NEW: the boundary commit runs after the staging merge, so the new SPEC §9 rows, BACKLOG rows and addenda are not
  covered by a gate record (the gate hashes the merged tree before them); `make check-docs` on the boundary commit
  is the integrator's call. Reason: the cache key is a tree, and changing it after the gate would turn every hit
  into a miss.
- B-NEW: `make usage` is not given a `--from` or `--to`; `USAGE_ARGS` passes through from the environment. Reason: the
  window is the integrator's decision at each boundary.

## Deferred clauses

- VAL-TOOL-006 is proven at `m4-integrate-m3`, the first integration that runs the target on the real repository. Here
  it was exercised only on throwaway repositories (`scripts/tests/integrate_test.sh`: hit, miss, red, resume, dirty
  tree, bad arguments, 55 checks); the real `make usage`, `make check` and the commit hooks of the repository did not
  run inside it.
- The model time of the integration against the fifteen-minute target of owner item 19 is measured by the integrator.

## Library

Line for `library/tooling.md`, section "Integrate target":

- `make integrate M=<m> FROM='<branch>...'` (`scripts/integrate.sh`, test `scripts/tests/integrate_test.sh` =
  `make check-integrate`, part of `make check-scripts`). Run it on a clean mission branch checkout. Output ends with
  `==== Boundary summary: <m> ... ====`, the block the integrator pastes. Inputs: `.git-trailers`,
  `scripts/templates/integrate-commit.txt` and `integrate-boundary.txt`, `reports/staging/*.md`. A red gate stops the
  run before usage and staging; fix with a new commit and run the same command again (merged branches are skipped).
  `CAPS='role=USD ...'` and `USAGE_ARGS='--from <ts> --to <ts>'` reach `make usage`. `INTEGRATE_DATE=YYYY-MM-DD`
  fixes the date (tests).
