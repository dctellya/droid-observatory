# Staging: m4-gate-key-and-ci-paths

Feature `m4-gate-key-and-ci-paths` (owner item 17 and B-056), worktree `observatory-wt/m4`, branch `wt/m4`.

## SPEC §9

- Row (F-NEW), rule 27 amendment (owner item 17): the gate cache is keyed by the CODE paths. `scripts/gate.sh` hashes the
  working tree (uncommitted and untracked files included) minus the excluded set `GATE_EXCLUDE_DIRS="build-log reports docs"` and
  every `*.md` outside `GATE_CODE_ROOTS="crates apps scripts config .github"`; the set is written once in the script and, with
  the reason, in `config/observatory.example.toml`. Records live in `reports/gate/<code_tree>/<name>.json` and keep
  `fingerprint` (never `key`: gitleaks and Droid-Shield read a `key` field holding 40 hex characters as an API key) and add
  `code_tree` and `tree` (the old key: build-log/ and reports/gate/ out only). `fingerprint` hashes the code tree, the toolchain
  and the command. A run that finds the code paths equal to a recorded gate prints `cached, proven at ...` then
  `gate <name>: HIT code tree <code_tree>` (and, when `tree` moved, that non-code paths changed since the proof); a miss prints
  `gate <name>: miss <code_tree>, running` then `gate <name>: MISS, code files that differ from the newest record (...)` and the
  `git diff --stat` between the newest record's `code_tree` and the current one, restricted to the code paths by construction.
  `scripts/gate.sh --diff <rev>` prints the same list from a revision. `hits.log` field 3 is the code tree. Evidence:
  `scripts/tests/gate_test.sh` (red `a7add22`, green `560143b`).
- Row (F-NEW), owner item 17 (c): `make integrate` uses the same key (`scripts/integrate.sh`, `6ab5efd`). It prints the gate's
  HIT or MISS lines and, on a miss, the summary reason is `gate.sh --diff <branch>` for each merged branch (code files only; it
  listed docs and reports before). Evidence: `scripts/tests/integrate_test.sh`, the `code paths` scenario (red `6bb4982`).
- Row (F-NEW), B-056 (owner item 17 (d)): `.github/workflows/mission-branch.yml` has `on.push.paths-ignore` (`reports/**`,
  `build-log/**`, `docs/**`, `*.md`, `ai/**/*.md`, `.factory/**/*.md`) and the `rust` job's `test` step runs
  `cargo test -p observatory-cli` only (the core tests run once, in `coverage`). `scripts/check-ci-jobs.sh` pins both. Warm
  measurement on `wt/m4` (run 40851767262, attempt 2, same commit as the cold attempt 1): 144 s against the 3-minute budget; the
  previous warm run (40222605337) was 221 s, so there are not two consecutive over-budget runs and the budget stays 3 minutes
  (`reports/ci/2026-10-29-m4-gate-key.md`).
- Row (F-NEW), the `[skip ci]` rule (F-137) ends: the path filter exists, so a report-, log- or docs-only push no longer needs
  `[skip ci]` in its subject; the marker stays harmless. A push that also touches a code path runs the checks whatever its subject.

## Decisions

None.

## BACKLOG

- Closed: B-056 (CI critical path: the trim is applied, `test` step 143 s to 19 s warm, run 40851767262; `coverage` (125 s) is now the
  critical path and splitting it further does not shorten the wall time).

## Deferred clauses

- The gate records of the M0 to M3 trees (`reports/gate/<old tree>/`) have no `code_tree` and are never a hit again; the gate prints
  "no earlier record" for a name whose only records predate the field. Not migrated: they are history, and the next gate run
  writes the new shape.
- A docs-only change is no longer re-linted by the local `make check` (markdownlint, lychee, the decisions index and
  `agents_md_test.sh` read paths the key leaves out) nor by the push CI; the pull request run (`gate.yml`, `workflow_call`) still
  runs all of it. This is the owner's trade (item 17).

## Library

- `scripts/gate.sh --key` prints the code tree (line 1) and, with a command, the fingerprint (line 2); the whole `tree` is in the
  record only. The record directory is the code tree, so `reports/gate/<key>/<command>.json` in the M4 rules reads as the code tree.
- Inside `scripts/gate.sh` the temporary index starts from `git read-tree HEAD`, drops `build-log` and `reports/gate`, adds the
  working tree, writes `tree`, then drops the excluded set and the Markdown outside the code roots and writes `code_tree`.
- `paths-ignore` cannot negate: Markdown outside the code roots is listed by directory in the workflow; a new top-level directory with
  Markdown is outside the code roots, so the script leaves its Markdown out of the key by itself but the workflow list needs an entry.
