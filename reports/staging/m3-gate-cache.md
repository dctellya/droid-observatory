# Staging: m3-gate-cache

Feature `m3-gate-cache` (rule 27, owner item 16), worktree `observatory-wt/m3`, branch `wt/m3`.
VAL-TOOL-001..004. Worker model line: `model = claude-sonnet-5-5`.

## SPEC §9

- Row (F-NEW): gate cache. `scripts/gate.sh <name> -- <command...>` keys a gate run by tree hash (working tree
  including uncommitted and untracked files, `build-log/` and `reports/gate/` excluded), tool version line and
  command string; a hit replays the stored verdict and exit code from `reports/gate/<tree>/<name>.json`,
  `--force` or `FORCE=1` reruns. `make check`, `make test`, `make lint` and `make coverage` go through it;
  `make check-gate-cache` is the self-test and `make check-scripts` (part of `make check`) runs it with the other
  shell tests. CI calls cargo directly and does not use the cache.
- Row (F-NEW): the key sequence in `library/tooling.md` is amended by one step. `git read-tree HEAD` brings the
  committed copies of `build-log/` and `reports/gate/` into the temporary index, and the `:(exclude)` pathspec of
  `git add -A` only stops working-tree changes, so committing a record or a build-log line changed the tree. The
  script runs `git rm -r -q --cached --ignore-unmatch -- build-log reports/gate` after `read-tree`. Validators
  reproducing the key by hand use the amended sequence (or `scripts/gate.sh --key`).

## Decisions

None. Alternatives considered: key by `shasum` of a file list (rejected: slower, no uncommitted-content rule for
free), CI use of the cache (rejected: a runner has no records, and an uncached CI guards against a stale one).

## BACKLOG

- B-NEW: wire the non-cargo gates (`check-docs`, `check-links`, `check-workflows`, `check-secrets`, `deny`,
  `machete`, `fmt-check`) through `scripts/gate.sh` with the tool's own `--version` line as the tool component.
  Reason: the owner item named check, test, lint and coverage only; `scripts/gate.sh` already accepts any name
  and takes the `rustc | cargo` line for `check test lint fmt-check deny machete coverage replay-fixtures`.
- B-NEW: add `make check-scripts` to the CI `docs` job. Reason: the CI workflows call targets one by one and
  this feature did not touch them.

## Deferred clauses

- `make coverage` was wired and checked with `make -n coverage`; it was not run (coverage runs alone at the
  integration, rule 23).
- The record fields follow the validation contract (`finished_at`, `seconds`, `exit_code`, `tool`); the feature
  text's `provenAt`, `durationSec`, `toolchain` are the same values under those names.

## Library

README paragraph for the Commands section (the integrator writes README):

> **Gate cache.** `make check`, `make test`, `make lint` and `make coverage` run through `scripts/gate.sh`. The
> script keys a run by the tree of the working directory (uncommitted and untracked files included, `build-log/`
> and `reports/gate/` excluded), the `rustc -V | cargo -V` line and the command string. A miss runs the command and
> writes `reports/gate/<tree>/<name>.json` (exit code, summary lines, seconds, UTC times, the Droid session);
> a hit prints `cached, proven at <time> by <session>` and the stored summary, exits with the stored code and
> runs nothing. `FORCE=1 make <target>` reruns, `scripts/gate.sh --key` prints the tree, and
> `make check-gate-cache` is the self-test. The records are committed, so a worktree at the same content hits.

Lines for `library/tooling.md`, section "Gate cache":

- The session in a record comes from `DROID_SESSION_ID`, then `FACTORY_SESSION_ID` (the Droid shell exports
  it; verified 2026-10-25 in a worker shell), then the newest `~/.factory/sessions/*/*.settings.json` by mtime,
  then `unknown`; the record's `session_source` names which. Override the directory with `GATE_SESSIONS_DIR`.
- Record fields: `name, tree, fingerprint, tool, command, exit_code, summary, seconds, started_at, finished_at,
  session, session_source`. A hit needs `fingerprint` (hash of tree, tool and command) to match, so a changed
  command or toolchain on the same tree and name is a miss that replaces the record. The field is not called
  `key`: gitleaks and Droid-Shield read `"key": "<40 hex>"` as an API key and block the commit of the records.
- Key sequence, amended: `GIT_INDEX_FILE=$tmp sh -c 'git read-tree HEAD && git rm -r -q --cached
  --ignore-unmatch -- build-log reports/gate && git add -A -- . ":(exclude)build-log" ":(exclude)reports/gate"
  && git write-tree'`.
- `make check` is itself a gate (`check`) that runs `make check-all`; `test` and `lint` inside it are gates too,
  so a `make check` after `make test` on the same tree replays the test verdict. `FORCE=1 make check` forces all
  of them.
- The duration is measured with `perl -MTime::HiRes` under `LC_ALL=C` (a comma locale otherwise breaks the JSON
  number); without perl it falls back to whole seconds.
