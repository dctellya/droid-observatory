# Process record: VAL-PROC-001 to 037 at the M5 boundary

Written by `final-process-record` (integrator, session `dd3305c1-5765-48cb-b79d-7b69b87ca91c`) on `mission/observatory-v1`, 2026-10-30,
after the m5 boundary (merge `44a4b47`, boundary commit `2bcca58`, head `6000939`) and the correction commit `20b90c9`. Every line below
was produced by the command shown, in the main checkout, on this branch; numbers recorded by earlier boundary reports are marked
`recorded` and name their file. No code was changed. Verdicts: **pass**, **pass with note**, **gap** (the literal contract text is not
met, the cause is named and has a BACKLOG row for the contract seat), **open** (needs the owner or a later feature), **not mine**
(assigned to `final-open-pr` or to a later validator).

## Summary

| Assertion | Verdict | One line |
|---|---|---|
| VAL-PROC-001 | open | `wt/m5` still exists (`git worktree list`); `final-open-pr` removes it after UT round 3 |
| VAL-PROC-002 | gap | 183 red commits, 14 production commits without an earlier red one (B-264); red runs read from hand-offs |
| VAL-PROC-003 | pass (recorded) | `make check` exit 0 on every integration, M5 real run 656.131 s on code tree `f0f20497` |
| VAL-PROC-004 | pass (recorded) | coverage 93.71 to 94.48 % lines, M5 94.08 % |
| VAL-PROC-005 | pass with note (recorded) | M5 smoke is UT round 2 flow `tauri`; the integrator ran no window |
| VAL-PROC-006 | gap | S literal checks (B-265); m0 to m5 pass |
| VAL-PROC-007 | pass | gitleaks fixtures and history both clean |
| VAL-PROC-008 | gap | 0 long subjects, 0 WIP, 2 commits without a parseable trailer (B-266) |
| VAL-PROC-009 | gap | m0, m1, m2, m4, m5 green; S red on both runs (recorded, fixed in the m0 pass); the m3 merge commit has no run |
| VAL-PROC-010 | pass | seven merges, each branch tip the second parent once |
| VAL-PROC-011 | open | M5 run 41045311362 is cold, 228 s, no warm run (F-339) |
| VAL-PROC-012 | pass | seven usage reports |
| VAL-PROC-013 | open | m3 and m4 have no readiness json (B-259, F-338) |
| VAL-PROC-014 | pass | §10 rows S, M0 to M5 (M5 written here); §9 rows F-337 to F-342 |
| VAL-PROC-015 | pass | README Honest time row for M5 |
| VAL-PROC-016 | pass with note | B-016, B-007 marked done here; B-008 open for M6 |
| VAL-PROC-017 | pass | `make check-decisions-index` exit 0 |
| VAL-PROC-018 | gap | entries 29, 32, 35, 36, 37, 39 and 40; run id and session ids thin in 37 and 39 (B-269) |
| VAL-PROC-019 | open (owner) | none of the seven screenshots exists; the owner supplies them (F-340) |
| VAL-PROC-020 | pass | no `force_push`, no amend, reset or rebase in any reflog |
| VAL-PROC-021 | pass | `origin/main` is `c8524c2`, 0 commits since |
| VAL-PROC-022 | pass | hooks path `.githooks`, hooks.json tracked, every hook-change commit has a decision or an exception |
| VAL-PROC-023 | pass with note | layout clean; the build-log was not committed at M4 and M5, committed here |
| VAL-PROC-024 | gap | `make check-file-size` exit 0; contract awk prints a package-lock (B-267) |
| VAL-PROC-025 | pass | `make check-todos` exit 0, grep prints 0 |
| VAL-PROC-026 | pass | 0 issues, 0 labels, no script, D-015 |
| VAL-PROC-027 | pass with note | settings unchanged; fallback models appear in five usage reports as recorded episodes |
| VAL-PROC-028 | pass with note | M5 overrun 71.27 against 30: owner told, cap moved to 190 (F-337) |
| VAL-PROC-029 | pass (recorded) | entry 32 and F-010 follow-ups; not re-derived here |
| VAL-PROC-030 | pass | D-030 go, rework 1, clean build 56.95 s |
| VAL-PROC-031 | pass | D-047, 182 under 200, test `ok` |
| VAL-PROC-032 | pass | no usage-throttle stop in M5; the one pause is a model fallback (F-341) |
| VAL-PROC-033 | gap | 23 shared-file commits on worktree branches, 14 not excepted (B-268) |
| VAL-PROC-034 | pass for M5 | scrutiny round 2 pass, UT round 4 pass; earlier boundaries recorded in their entries |
| VAL-PROC-035 | pass with note | inputs recorded at their boundaries; reference CSV absent at M4 to M5 with the stall flag raised |
| VAL-PROC-036, 037 | not mine | `gh pr list` prints 0 now; `final-open-pr` opens the PR |

## VAL-PROC-001

```sh
git worktree list; git branch --list 'wt/*'
```

```text
/Users/dct/code/EllyaOrg/projects/dalimaray/droid-observatory  6000939 [mission/observatory-v1]
/Users/dct/code/EllyaOrg/projects/dalimaray/observatory-wt/m5  d62c256 [wt/m5]
  wt/m0  wt/m1  wt/m2  wt/m3  wt/m4  + wt/m5  wt/s
```

The worktrees of s to m4 are removed and their branches kept. `wt/m5` is still there: the orchestrator heartbeat and UT round 3 used
it, and the feature `final-open-pr` removes it and creates no new one.

## VAL-PROC-002

```sh
python3 <(scan)   # per milestone: git rev-list --no-merges --reverse <merge>^1..<merge>^2, a `test(...): red` subject
                  # counts as red; a feat/fix/refactor/perf commit touching crates/*/src or apps/*/(src|src-tauri/src|ui/src)
                  # needs an earlier red commit not yet matched by another production commit
```

```text
s  commits=26 red=11 prod-green=11 without-red=1   e6ea1aa feat(workspace): add cli, ui and Tauri shell crates
m0 commits=24 red=9  prod-green=7  without-red=0
m1 commits=52 red=19 prod-green=22 without-red=3   74f29fc dac2cd4 bbe3d70
m2 commits=67 red=25 prod-green=32 without-red=7   a2923e5 acd6cad f0d722f 73a3203 1c6d998 b16185d 0eade53
m3 commits=92 red=27 prod-green=24 without-red=2   a4413c7 995fa86
m4 commits=186 red=63 prod-green=34 without-red=0
m5 commits=79 red=29 prod-green=22 without-red=0
```

The red run is in the worker hand-off (`verification.commandsRun`), not in `build-log/` (the hook records `exit_code` as `null` for
calls inside a Script, so a failing `cargo test` is not visible there). Count over the mission's hand-offs of implementation
features: 100 hand-offs, 96 record a red or failing run, 4 do not (`m1-view-models-core` twice, the recovered hand-off, and the
documentation fix `m2-fix-d032-alternatives` twice). Sample: M5 red `d41c46e` (09:41 local) precedes green `529a850` (09:46). Gap: the 14
commits are shell and UI wiring, a config key, a clippy fix and complexity refactors; BACKLOG B-264.

## VAL-PROC-003, 004, 005 (recorded)

Pasted from the boundary reports, listed in SPEC §10 (the row of each milestone): `make check` exit 0 on `38e5617`, `f66a9f5`,
`c2fb42a`, `7385b62`, `a3adb9b3` (m3), `e8f7cea9` (m4), `f0f20497` (m5, real run 656.131 s, lint 197.286 s, test 333.934 s).
Coverage TOTAL lines: S 94.48 %, M0 94.21 %, M1 93.89 %, M2 94.20 %, M3 94.21 %, M4 93.71 %, M5 94.08 % (`reports/gate/f0f20497…`,
`reports/integrate-long/`). On the merged tree this session ran `make check-contract`:

```text
contract checks: 19 ok, 12 skipped, 0 FAIL in 7.6s
```

(the 12 skips are the network checks and the worktree-only ones, run below by hand.) Tauri smoke at M5: UT round 2 flow `tauri`
(VAL-METRIC-013 and VAL-TELEM-009 passed); the integrator started no window.

## VAL-PROC-006

```sh
# per milestone, at the worktree tip, with mb = git merge-base <merge>^1 <merge>^2
s : touched SPEC/README/BACKLOG/rust-toolchain: none; docs/decisions/README.md: TOUCHED (537ed0c, D-011)
    src/store, src/metrics, src/control: absent; grep -c rusqlite Cargo.lock: 2; git log origin/main..main: 0
m0: view diff 0; metrics, control, ingest/{watch,rpc,git,github,forge}.rs and the five UI component dirs: absent
m1: host.rs, rpc.rs, github.rs, forge.rs: absent; control/: absent; command table hits: 0; ureq in Cargo.lock diff: 0
m2: github.rs, forge.rs, git.rs, control/stall.rs, control/merge.rs, metrics/: absent; commands human_action/gate/merge: 0
m3: metrics/, github.rs, git.rs, forge.rs, control/merge.rs: absent; commands gate/merge/metrics/export: 0
m4: metrics diff 0; origin/main c8524c2641c00376340640859bb5381763c59ba6; ruleset 25636994 sha256 784a23cd… before the run
    (scripts/contract-checks/main-gate.json) and now: identical
m5: g5-frozen-fixtures ok VAL-METRIC-022 VAL-PROC-006
```

Gap (S only): `rusqlite` is in `Cargo.lock` at `wt/s` because the architecture §2 scaffold declares the optional feature
`store = ["dep:rusqlite"]` (no store module exists), and the generated decisions index was regenerated for D-011. The S validators
passed both; BACKLOG B-265 rewords the check. The `lsof` check of the m1 app is VAL-BOARD evidence of the m1 round, not re-run here.

## VAL-PROC-007

```text
gitleaks detect --no-git --no-banner --redact --source fixtures/streams   exit 0   scanned ~68.21 MB   no leaks found
make check-secrets   exit 0   621 commits scanned   scanned ~10.08 MB   no leaks found
```

## VAL-PROC-008

```text
git log --format=%s c8524c2..mission/observatory-v1 | awk 'length($0) > 72' | wc -l            0
git log --format=%s c8524c2..mission/observatory-v1 | grep -Ec '^(WIP|wip|fixup!|squash!)'      0
longest subject 72 characters; commits in range 614
per-commit trailer loop: MISSING 2
  bb7be45 docs(design): ui kit v2 for the M3 and M4 screens   (owner commit 2026-10-25, trailer `Co-Authored-By: Claude Fable 5.1`, F-069)
  8ab88e2 docs(decisions): D-032 names the Lane::home(role) alternative   (stray heredoc tail, AGENTS.md "stays, forward only")
```

`scripts/contract-checks/g6-commit-format.sh` (the form of record) prints `ok` because its range starts at `git-baseline.txt`
(`3ccf3aa`). Gap against the literal range: B-266. History is not rewritten (rule 12).

## VAL-PROC-009, 010

```text
s  38e5617 run 39919592562 failure; the second run 39920664605 (2ba8fb0) also failure (recorded in reports/ci/2026-10-23-s.md: the Linux runner lacked glib-2.0, D-022; fixed in the m0 pass, cc05aa5)
m0 f66a9f5 39953923391 success     m1 c2fb42a 40061513431 success     m2 7385b62 40121391298 success
m3 0bf60b0 (no run on the merge commit; the boundary commit f6dddaa has 40222216751, reports/ci/2026-10-25-m3.md)
m4 3f38afe 40964294625 success (attempt 2)     m5 2bcca58 41045311362 success; jobs machete success, coverage success, mission-checks success
first-parent merges: 44a4b47 (m5), 3f38afe (m4), 0bf60b0 (m3), 7385b62 (m2), c2fb42a (m1), f66a9f5 (m0), cc05aa5 and 38e5617 (s)
second parent of each equals `git rev-parse wt/<m>` exactly once: s cd53a3c, m0 88866ef, m1 97b3b75, m2 36a042e, m3 ce0b143, m4 c9ec1f3, m5 d62c256
```

## VAL-PROC-011, 012, 013

```text
ls reports/ci:        2026-10-23-s 2026-10-23-m0 2026-10-24-m1 2026-10-24-m2 2026-10-25-m3 2026-10-29-m4 2026-10-30-m5 (.md)
ls reports/usage:     s m0 m1 m2 m3 m4 m5 (.md)
reports/readiness/*.json at s m0 m1 m2 m5, each `jq empty` exit 0; m3 and m4: none
```

VAL-PROC-011 is **open**: the only M5 run (41045311362, `2bcca58`) is cold, 228 s against the 180 s warm budget; every later commit
carries `[skip ci]`, so no warm run exists, and no coverage-to-final-PR decision file exists (D-016 is the 80 % gate only). The
`gate` run of the final PR is the next measurement. VAL-PROC-013 is **open**: the owner decides at the M5 boundary to waive the m3
and m4 files or to run `/readiness-report`; nothing is backfilled (F-338, B-259).

## VAL-PROC-014, 015, 016, 017

```text
grep -n '^| S \|^| M[0-5] ' SPEC.md    → rows S, M0, M1, M2, M3, M4 (existing) and M5 (line 1074, written here), no `—` cell
SPEC §9: F-330 to F-336 (integrator, final-integrate-m5), F-337 to F-342 (this record)
README.md: Honest time table has a row for every milestone, M5 added here from reports/usage/m5.md
BACKLOG.md: B-016 done 2026-10-23, B-005, B-014, B-006, B-012 done, B-007 done 2026-10-29 (both marked here), B-008 open (reference CSV, M6),
            B-015 row updated; new rows B-262 to B-269
make check-decisions-index   exit 0
```

## VAL-PROC-018, 019

```text
grep -n '^## Entry' ai/session/2026-10-22-session.md   → 29 s, 32 m0, 35 m1, 36 m2, 37 m3, 39 m4, 40 m5 (written here)
```

Each entry names its integration sha (checked by script: `38e5617`, `f66a9f5`, `c2fb42a`, `7385b62`, `0bf60b0`, `3f38afe` present). Run
ids are present in entries 29, 32, 35, 36 and 39; entry 37 has none (the m3 run is 40222216751) and entries 37 and 39 name two session
ids each (the milestone tables of `reports/usage/<m>.md` list all). Entry 40 repairs this by pointing at those files (B-269).

VAL-PROC-019 is **open, an owner item**. The expected files do not exist (`test -f` exits 1 for each of the seven):

```text
/Users/dct/code/EllyaOrg/projects/dalimaray/droid-observatory/reports/screenshots/2026-10-23-s.png
/Users/dct/code/EllyaOrg/projects/dalimaray/droid-observatory/reports/screenshots/2026-10-23-m0.png
/Users/dct/code/EllyaOrg/projects/dalimaray/droid-observatory/reports/screenshots/2026-10-24-m1.png
/Users/dct/code/EllyaOrg/projects/dalimaray/droid-observatory/reports/screenshots/2026-10-24-m2.png
/Users/dct/code/EllyaOrg/projects/dalimaray/droid-observatory/reports/screenshots/2026-10-25-m3.png
/Users/dct/code/EllyaOrg/projects/dalimaray/droid-observatory/reports/screenshots/2026-10-29-m4.png
/Users/dct/code/EllyaOrg/projects/dalimaray/droid-observatory/reports/screenshots/2026-10-30-m5.png
```

The repo-root `board-browser-m0.png` was never the owner's: it is the round-1 validator's browser capture (owner, 2026-10-30
12:30: "I didn't take it"). No image is invented to fill a path; `ffmpeg ... signalstats` has nothing to read until the owner adds
the files with the M6 boundary (F-340).

## VAL-PROC-020, 021

```text
reflog grep 'commit (amend)|reset:|rebase' on mission/observatory-v1 and wt/s wt/m0 wt/m1 wt/m2 wt/m3 wt/m4 wt/m5: empty for all eight
gh api repos/dctellya/droid-observatory/activity?ref=refs/heads/<ref>: activity types only
  mission/observatory-v1 {branch_creation:1, push:30}   wt/s {branch_creation:1}   wt/m0 {1, push:5}   wt/m1 {1, push:1}
  wt/m2 {1, push:1}   wt/m3 {1, push:1}   wt/m4 {1, push:3}   wt/m5 {branch_creation:1}   (no force_push, no other type)
git rev-parse origin/main  c8524c2641c00376340640859bb5381763c59ba6
gh api "repos/dctellya/droid-observatory/commits?sha=main&since=2026-10-22T14:45:41Z" --jq length  0
```

## VAL-PROC-022

```text
git config core.hooksPath → .githooks (repository); the `m5` worktree prints .githooks (the others no longer exist)
git ls-files .factory/hooks.json → .factory/hooks.json
jq … hooks.json → guard-git.sh, officer-decides.sh (PreToolUse), log-tool-call.sh (PostToolUse, twice for the Pre and Post events)
hook-change commits since c8524c2: f13ce91 7fef46e 060698a a4b4646 (D-041, D-042, D-043 name them); 545485d 6697945 19c4ad1 6800f45
  (listed in scripts/contract-checks/exceptions.txt); `g6-hooks-config ok`, `g4-hooks-json ok VAL-PROC-022`
```

## VAL-PROC-023

```text
git diff --stat <prev> <sha> -- build-log/ | tail -1   (layout grep -Evc → 0 for all seven)
s 15 files, m0 60, m1 62, m2 2, m3 115, m4 2, m5 0
```

The layout is clean. The M5 pass added no file: the build-log was last committed at `aa91578` and `d7f2660` (2026-10-25). Gap
repaired in this record by `chore(build-log): tool-call logs through the M5 record [skip ci]` (the files stay exempt from the size
check); B-269 proposes committing `build-log/` with every boundary commit.

## VAL-PROC-024, 025, 026

```text
make check-file-size   exit 0   (largest non-exempt: docs/spec/section-9-archive.md 184900, reports/usage/m4.md 195686; SPEC.md 169408 after this record)
contract awk (no package-lock exemption): apps/observatory-app/e2e/package-lock.json 252861   → gap, B-267
make check-todos       exit 0   git grep TODO|FIXME without (#N|B-N): 0 lines
gh issue list --label backlog --state all: 0     gh label list | grep -c '^backlog$': 0     git ls-files scripts/gen-backlog.sh: (empty)
SPEC §8: "hybrid backlog … withdrawn", docs/decisions/D-015-backlog-is-backlog-md-only.md
```

## VAL-PROC-027

```text
jq -r '.missionOrchestratorModel, … ' .factory/settings.json
claude-fable-5.1  high  claude-sonnet-5-5  high  claude-opus-5-5  high
git log c8524c2..mission/observatory-v1 -- .factory/settings.json   (empty)
```

The settings never changed. `kimi-k3` appears in `reports/usage/s.md`, `m0`, `m2`, `m4` and `m5` (and `gpt-6.1-sol` in `m2` and `m4`):
these are Model-Router fallback episodes the reports name (D-018, SPEC §9 F-046, the F-057 rows, F-341 for M5), each with the owner's
answer recorded; none is a silent change. `claude-haiku-4-5-20251001` is the declared scratch model (F-042).

## VAL-PROC-028

Cap against spent per milestone, from `reports/usage/<m>.md`: S 13.21 of 40; M0 49.05 of 60; M1 110.38 of 110 (over by 0.38);
M2 120.61 of 50; M3 210.98 of 110; M4 253.73 of 200; M5 163.57 of 190 (under, 26.43 left). Overruns M1 to M4 have owner answers
(F-055, F-148, D-038, D-043). The orchestrator line of M5 is 71.27 USD-eq (own session: input 93113, output 290930, cache read
81143534, cache write 2906765) against the 30 per milestone line: **over**. The owner was told the figure (63.03 at 09:44Z) in chat at
about 09:55Z and chose to raise the cap to 190 and finish M5 (row 80, `MD/contract-work/m4-boundary-questions.md`; D-048). SPEC §9
F-337 records it; the line is a stop-and-decide line under R-055 and D-038, not a wall.

## VAL-PROC-029, 030, 031

```text
VAL-PROC-029: entry 32 ("Open for the owner at the M0 to M1 boundary: re-caps for M1 to M5") holds the proposal; the owner's answer is
              SPEC §9 F-031 (M1 110 / M2 50 / M3 35 / M4 75 / M5 105, orchestrator inside) and entry 33 (recorded, not re-derived)
VAL-PROC-030: docs/decisions/D-030-leptos-go.md  "Rework sessions: 1 (limit 3)", clean clone trunk build 56.95 s and 61.04 s
VAL-PROC-031: docs/decisions/D-047-telemetry-series-allocation.md  "182 worst case under `max_series` 200"
              reports/gate/f0f20497…/test.tests: ok exported_series_count_is_under_budget_on_worst_case_labels
```

## VAL-PROC-032

No usage limit or cap stopped the M5 run. `reports/usage/m5.md` `### Throttled` reads `throttled: 0.000 s = 0.00 min`. The one pause
of the window (`mission_paused` 2026-10-30T04:15:43.965Z to `mission_resumed` 06:38:43.697Z, 143.0 min) followed the first usage
limit of the night, when a seat fell back to `kimi-k3`; it is an owner pause with the model check on resume (F-341), not a
`throttled` interval, and `.factory/settings.json` stayed as VAL-PROC-027 shows. The owner's cap answer of 09:55Z is verbatim in
row 80 and D-048; entry 40 repeats it.

## VAL-PROC-033

```text
per worktree branch: git log <mb>..wt/<m> -- README.md BACKLOG.md docs/decisions/README.md reports/usage reports/readiness reports/ci
s 537ed0c | m0 e1074bb | m1 c61d3a4 545485d | m2 f5fedae 6798f9b 7998f2b 94ddb12 2e48f92 | m3 cf9651d fbb7685 6330b88 c8c59e7
m4 cd9c5cf b4769e2 fa9b855 253e95a aa56596 b82fac9 1cd244c | m5 4e046cf 8bd6a7d 4b40a09
SPEC rows changed in a worktree (`git diff … -- SPEC.md | grep -E '^[+-]\| (F-[0-9]{3}|S |M[0-5] )'`): 0 for all seven
```

23 commits; the nine of m4 (six) and m5 (three) are in `scripts/contract-checks/exceptions.txt`, 14 are not (generated decisions
index, one README line, usage regenerations). The index is forced by `check-decisions-index` in a worktree that adds a decision.
Gap: B-268. No §9 or §10 row was written in a worktree.

## VAL-PROC-034, 035

```text
validation/m5/scrutiny/synthesis.json          round 2  verdict pass  blocking []
validation/m5/user-testing/synthesis.json      round 4  verdict pass
```

M5 is the only boundary re-read here; the earlier boundaries' lines are in entries 29 to 39 (the contract's exact phrase
"blocking findings open: 0" is not used verbatim there). The owner inputs sit at their boundaries: re-caps at m0 (entry 32), the
F-028 answers (entry 33), the M1 go/no-go (entry 35, D-030), the write-token question at m3 to m4 (entry 37 points to SPEC §9 F-136, Q1 to Q7 and the
write-token question; the PAT stays read-only, D-014, answers in `MD/owner-feedback/2026-10-25-m3-boundary.md` of the mission directory), the reference CSV at m4 to m5 (entry 39: missing, stall flag raised, VAL-METRIC-001 and 003 deferred to M6,
B-253). The final-PR `gate` result belongs to `final-open-pr`.

## VAL-PROC-036, 037

```text
gh pr list -R dctellya/droid-observatory --base main --head mission/observatory-v1 --state all --json number --jq length   0
```

The pull request does not exist yet; `final-open-pr` opens it and runs these two assertions.

## Forward note (2026-10-30): ruleset hash

Non-blocking, no line above changes. The m4 line of the VAL-PROC-006 section quotes `ruleset 25636994 sha256 784a23cd… before
the run (scripts/contract-checks/main-gate.json) and now: identical`. The committed copy and the live ruleset both hash to
a different value:

```text
shasum -a 256 scripts/contract-checks/main-gate.json   7ddea4844e2d7f2701a3734cc2ab9f053e3bd41492e2b8a48b5d4bbbf0c8a8e7
gh api repos/dctellya/droid-observatory/rulesets/25636994 | shasum -a 256   7ddea4844e2d7f2701a3734cc2ab9f053e3bd41492e2b8a48b5d4bbbf0c8a8e7
cmp <live> scripts/contract-checks/main-gate.json   exit 0 (1160 bytes each)
```

The record's `784a23cd` is not reproduced by either; the live and committed copies agree with each other (`7ddea484`), so
the "identical" claim holds and the quoted prefix is the record's error. How `784a23cd` was computed is not recorded.
