# droid-observatory

A Rust desktop app that watches a Factory Mission build a business feature: which agent is on
what, whether each milestone is validated, where time and tokens go, and the one gate that
ships it. It is a mission watcher: it measures the waste and effectiveness (cycle time, cost)
of the engineering cycle from the spec to the pull request's checks going green. Lanes are the
mission's seven phases; cards are milestones; roles are colour; the human's interventions are
overlays; time, waste, cost and model drift are the Panel. What happens after the merge
(release, observe, deploys, flags) belongs to the delivery platform and is out of scope: the
last event the app records is `handed_to_cd`.

**Status:** mission in progress. The walking skeleton (S), the stream capture and mapping milestone (M0), the
board in replay and attach (M1, ai/RULES.md rule 2, SPEC §3) and the host (M2: launch form, hosted `droid exec` over
stream-jsonrpc with pause, message and answer, live output per agent, D-033, D-034) and the controls (M3: the six human
actions, the stall flag, the cycle template as data D-035, the end-to-end harness D-037, the gate cache and `make usage` /
`make integrate`) are integrated on `mission/observatory-v1`: a Cargo
workspace, the canonical `Event`, mapping v1 with its golden file (seven phase lanes, D-024), a SQLite pointer
store, `observatory-cli replay` and `inspect`, view models, the attach watcher, the usage-log source (D-029), a
Leptos board with Details and agents views and a Tauri host that attaches to live sessions. The Leptos go/no-go at
M1 is **go** (D-030). Dogfooding started at the M1 integration: `make app-dev` from this checkout lists this
mission's own orchestrator session on the board (`reports/usage/m1.md`, session log entry 35). Host, intervention,
gate and metrics follow in M4-M5 (SPEC §3); the mission continues into M4 under the 1350 USD-eq line (owner decision, 25-oct, D-038; the cost was past the old 950 line, `reports/usage/m3.md`).

## What it will show

| Lane | Holds |
|---|---|
| 1 Plan | prompt, plan, milestones, task assignment, replans, the human's answers |
| 2 Contract | the validation contract and the spec versions |
| 3 Build | one sub-lane per worker: files, tests, commits |
| 4 Scrutiny | findings, blocking vs non-blocking, rework |
| 5 User testing | QA scenarios against the built thing |
| 6 Integrate | worktree landings on `mission/<name>`, gate commands, the PR opened |
| 7 Gate | the PR's checks and attestations, the review, `handed_to_cd` |
| Panel (above the lanes, not a lane) | engineering cycle time, waste (8 causes, shown as 5 groups), cost per role, drift against the declared models |

Human interventions (and, later, an officer's actions (D-035)) are overlay markers on these lanes. Two
clocks are named and never confused: the engineering cycle time (`feature.defined` to
`handed_to_cd`) is the app's; the lead time to production is the delivery platform's, starting at
the same merge sha.

Full definition: [ai/prd/droid-observatory-prd.md](ai/prd/droid-observatory-prd.md).
Spec: [SPEC.md](SPEC.md) (approved 2026-10-22; the mission plans from it).

## From a clean clone

Requirements: the Rust toolchain via [rustup](https://rustup.rs) (version pinned in
`rust-toolchain.toml`), and the tools `make doctor` lists.

```sh
git clone git@github.com:dctellya/droid-observatory.git
cd droid-observatory
make doctor      # which tools are missing and how to install them
make hooks       # enable the git hooks (once)
git config commit.template .gitmessage   # commit message shape (once; local config, not cloned)
make check       # every light check the mission-branch CI runs
cargo run -p observatory-cli -- replay fixtures/streams/2026-10-22-observatory-v1-m0.jsonl
                 # root file and its child sessions: 1279 events over 8735 records, ends with `unknown events: 0`
                 # (the S fixture gives 661 events); `--root-only` replays the root file alone (M0 474, S 364)
```

`make help` lists all targets. The same targets run in CI, so a documented command is a
command the gate runs.

### The board (UI)

The Leptos UI is a Trunk/WASM build in `apps/observatory-app/ui`. From a clean clone, with the
toolchain above and `trunk` installed (`make doctor` lists it):

```sh
cd apps/observatory-app && trunk build --config ui/Trunk.toml
                 # writes apps/observatory-app/ui/dist/: index.html, one *_bg.wasm, one *.js
                 # (M1 go/no-go, D-030: 56.95 s on a clean clone with its own target directory)
```

Two ways to see the board (SPEC §6):

- Browser host: `make ui-serve` (or `trunk serve --config apps/observatory-app/ui/Trunk.toml --port 3141
  --open false` from the repository root), then open <http://localhost:3141> and pick the M0 capture
  `fixtures/streams/2026-10-22-observatory-v1-m0.jsonl` (the root file; its `.children/` come with it)
  in the file picker: seven phase lanes, the Panel, the unmapped tray and Details. The browser host
  replays files only.
- Tauri window: `make app-dev`. The window replays the same way and adds Attach: at start it scans
  `~/.factory/sessions` (the files that grew in the last `[ingest].startup_scan_hours`, 24) and tails
  them, so a mission running in a terminal appears on the board within a second. A session is
  named by the first `--tag` you gave it, else by its session id. The library has an open-by-path
  field for a replay file (there is no native file dialog yet, B-044). Events go to the SQLite
  index at `[store].path` (`~/.droid-observatory/observatory.sqlite` by default), pointers only.

## Commands

| Target | What it does | Where it runs |
|---|---|---|
| `make check` | all light checks below | mission branch, gate |
| `make check-docs` | markdownlint on every `.md` | mission branch, gate |
| `make check-links` | lychee link check | mission branch, gate |
| `make check-workflows` | actionlint | mission branch, gate |
| `make check-file-size` | tracked files under 200 KB (fixtures exempt) | mission branch, gate |
| `make check-todos` | debt markers must cite `#issue` or `B-nnn` | mission branch, gate |
| `make check-decisions-index` | `docs/decisions/README.md` is generated and current | mission branch, gate |
| `make check-secrets` | gitleaks over the repository | mission branch, gate |
| `make fmt-check` / `make lint` / `make test` / `make deny` / `make machete` | rustfmt, clippy `-D warnings`, tests, cargo-deny, unused dependencies | mission branch, gate |
| `make replay-fixtures` | every `fixtures/streams/*.jsonl` through `observatory-cli replay`, child sessions included | mission branch, gate |
| `make coverage` | line coverage of `observatory-core` (run alone); floor 80 % from M0 | mission branch (job `coverage`) |
| `make app-dev` / `make app-build` | Tauri dev window / macOS `.app` bundle | local |

## Configuration

- `config/observatory.example.toml`: every runtime key with its default and reason. Copy to
  `~/.droid-observatory/config.toml`.
- `config/prices/<date>.toml`: the dated price table that turns tokens into money. Each cost on
  the board names the table it used.
- `.env.example`: environment variables (`GITHUB_TOKEN`, `FORGE_LIVE`, the CI-only OTLP variables). Secrets go
  in the OS keychain on the desktop; the environment is the fallback and what CI uses.

## Environment and secrets

See `.env.example`. Secrets come from the OS keychain (`droid-observatory/<name>` entries) or the environment, never from the
repository, the store or logs. The forge (the Gate lane's GitHub source) is off unless `GITHUB_TOKEN` is set or the owner starts the
app with `FORGE_LIVE=1`, which lets it read the keychain item `droid-observatory/github-token` (`FORGE_LIVE=1 make app-dev`, or
`GITHUB_TOKEN="$(gh auth token)" make app-dev` for the token without the keychain). `FORGE_LIVE=1` is the owner's launch from his
own terminal: no process a worker starts reads the keychain (F-218, F-219). The token is a read-only fine-grained PAT, so the
Merge button surfaces the `403` and the owner merges by hand (D-014).

## Demo branch pair (test infrastructure)

The Gate lane is exercised against a branch pair in this repository (D-040): `demo/base` is long-lived test infrastructure and
stays; it carries the ruleset `demo-gate` (PR required, required check `demo-gate`, no force push) and the workflow
`.github/workflows/demo-gate.yml`, which fails while the label `demo:red` is on the pull request. PRs come from `demo/*` into
`demo/base`; every `demo/*` branch other than `demo/base` is deleted after its run (a `gh` step, not an observed event).
After the M4 validation, on 2026-10-29T22:20:16Z:

```sh
gh api repos/dctellya/droid-observatory/branches --paginate --jq '.[].name' | grep '^demo/'
```

```text
demo/base
```

Pair PRs of the M4 validation: #29, #32, #33 and #34 (merge `0a853d9`).

## Running it as a mission

Models per role (ai/RULES.md rule 19, `.factory/settings.json`, D-006):
orchestrator `claude-fable-5.1`, workers `claude-sonnet-5-5`, validators `claude-opus-5-5`.

```sh
droid exec --mission --auto high \
  --model claude-fable-5.1          --reasoning-effort high \
  --worker-model claude-sonnet-5-5  --worker-reasoning-effort high \
  --validator-model claude-opus-5-5 --validator-reasoning-effort high \
  --output-format text \
  -f SPEC.md
```

Reasoning effort per role is `high` for all three (SPEC §2). The mission works on
`mission/<name>`, one worktree per milestone, and ends in one pull request into `main`.

## Running a boundary

The integrator lands a finished milestone with one target, from the mission branch checkout with a clean tree
(only `build-log/` may differ):

```sh
make integrate M=m3 FROM=wt/m3       # FROM takes several branches, separated by spaces
```

`scripts/integrate.sh` does, in order: a `git merge --no-ff` of each branch with the message rendered from
`scripts/templates/integrate-commit.txt` and the trailers of `.git-trailers` added by `git interpret-trailers`; the final
gate through `make check` (`scripts/gate.sh`: a printed cache hit when the merged tree is the tree a worker or validator
already proved, a real run on a miss, an abort with the summary when red, never a rerun); `make usage M=m3`
(`CAPS` and `USAGE_ARGS` pass through); the merge of every `reports/staging/<feature>.md` into `SPEC.md` §9 (next free
`F-NNN` rows), `BACKLOG.md` (next `B-NNN` rows) and the decision files (an addendum per amendment), after which the
staging files are deleted and `make decisions-index` runs; the boundary report
`reports/readiness/<date>-<milestone>.md` (merges, the gate numbers copied from `reports/gate/<tree>/*.json`, deferred
clauses, library notes); one boundary commit; and the boundary summary the integrator pastes. A failed step leaves the
merges in place: fix with a new commit and run the target again (a merged branch is skipped). Judgement edits
(`SPEC.md` §10, `README.md`, the Honest time table) stay by hand. `make check-integrate` is the self-test.

## The gate

Two workflows, one required check:

- `mission-branch.yml` on `mission/**`: docs, links, workflows, secrets, and (once code
  exists) fmt, clippy, tests, replay fixtures, cargo-deny. Aggregate job `mission-checks`.
  Budget: three minutes warm.
- `gate.yml` on pull requests to `main`: everything above plus Semgrep, Trivy, Checkov on the
  workflows, an SBOM, the macOS Tauri bundle (`bundle.yml`) with build provenance and cosign
  signatures. Semgrep and Trivy findings are uploaded to GitHub code scanning, which is also
  enabled in default setup. Aggregate job `gate`, the only check the `main` ruleset
  requires. It never deploys.
- `release.yml` on tags `v*` pushed by the human to a commit on `main`: builds the same
  bundle and creates the GitHub release with notes generated from PR labels
  (`.github/release.yml`, D-009). Runbook: `docs/runbooks/release.md`.

Branch rules on `main`: pull request required, `gate` required, no force pushes, no admin
bypass, zero required approvals (the human merges by hand, the gate is the review; the merge is
the app's last event, `handed_to_cd`).

## Observability

The app exports the Panel's mission series (`observatory.*`) over OTLP to a local collector
(`[telemetry].endpoint`, default `http://localhost:4318`, D-046); the collector is configured for the sink (Grafana Cloud, a local
Grafana stack or any provider), so the app holds no provider credential. Off by default; the app works fully offline. An optional
header comes from the keychain item named by `[telemetry].headers_keychain_item`, never from the repository. CI telemetry
(B-002) stays deferred.

## Repository layout

```text
ai/            PRD, rules, spec template, session logs (the human-agent record)
.factory/      mission model settings, hooks: git guard and tool-call logger
.github/       workflows (mission-branch, gate, bundle, release), CODEOWNERS, templates
.githooks/     pre-commit, commit-msg, pre-push (enable with `make hooks`)
.devcontainer/ Linux container with every `make check` tool (core work; the window needs macOS)
config/        example config and dated price tables
docs/decisions/ one file per decision; README.md is generated
docs/runbooks/  red gate, fix cycle, stalled mission, hooks disabled, release
docs/spec/     SPEC §9 rows F-001 to F-160 (S to M3), archived verbatim
build-log/     one JSON line per agent tool call, by day and session
reports/       readiness reports, usage and waste per milestone (pasted from the tool)
scripts/       the checks the Makefile calls
BACKLOG.md     deferred items with their reasons
```

## Honest time

Rule 10: real time and usage per milestone, stated here as the work lands.

| Phase | Wall time | Sessions | Notes |
|---|---|---|---|
| Ideation → PRD v0.2 | ~1 h 40 m | 1 Droid session | two review rounds; log in `ai/session/2026-10-22-session.md` |
| Repo readiness | ~2 h | same session | readiness Level 1 (5/65) → Level 3 (31/65); see `reports/readiness/` |
| S walking skeleton | 63.9 min of assistant time in 8 parent sessions (first worker 2026-10-22 16:49Z, scrutiny done 17:39Z, user testing finished 2026-10-23 06:40Z after an overnight usage limit) | 6 workers, scrutiny and user-testing validators with their 10 children | 13.21 USD-eq of the 40 cap on workers and validators; the orchestrator's share is not exposed (`reports/usage/s.md`) |
| M0 capture and mapping | 162.9 min of assistant time in 14 parent sessions (first worker 2026-10-23 07:41Z, user testing round 1 finished 12:50Z; a 5-hour usage limit paused the mission at 11:03Z) | 11 workers, scrutiny (two rounds) and user-testing validators with their children | 49.05 USD-eq of the 60 cap priced so far (workers, validators, orchestrator subagents); the orchestrator's own delta is 66.87 USD-eq against its cap of 32 (`reports/usage/m0.md`) |
| M1 board, replay, attach | 342.0 min of assistant time in 23 parent sessions (first worker 2026-10-23 12:29Z, last validator finished 2026-10-24 04:08Z; a 4 h 13 min owner pause and two 5-hour usage limits, 255.4 min in total, are inside that wall time) | 17 workers (this integration included), scrutiny and user-testing validators with their 25 children | 110.38 USD-eq of the 110 cap on workers, validators and orchestrator subagents; the orchestrator's own delta is 115.74 USD-eq against its cap of 22 (`reports/usage/m1.md`) |
| M2 host | 871.95 min of wall clock from the first worker (2026-10-24 05:04:43Z) to the report time 19:36:40Z, of which 121.3 min are one `unrecoverable_usage_402` throttle; episode 4 (Model-Router fallback to `kimi-k3`, SPEC §9 F-046) voided the sessions listed in that row | 9 workers (this integration included), validators in five scrutiny and four user-testing rounds | 120.61 USD-eq of the 50 cap on workers, validators and orchestrator subagents; the orchestrator's own M2 spend is over its 30 USD-eq stop (118.84 USD-eq on the log's fable turns); record mission-to-date 694.66 USD-eq, 110.3 % of the 630 hard stop (`reports/usage/m2.md`) |
| M3 controls | 620.1 min of wall clock from the first worker (2026-10-25 07:59:07Z) to the end of the usage window 18:19:14Z, of which 15.64 min are one `unrecoverable_usage_402` throttle (the F-057 verification, SPEC §9 F-132) | 19 workers (this integration included), two scrutiny rounds and three user-testing rounds (a fourth session, `1653b1f7`, was stopped by the 402) | 210.98 USD-eq in the window against the cap 110 (over); the orchestrator's own 68.81 USD-eq against 30 (over); mission to date 990.09 USD-eq, past the 950 stop-and-decide line (`reports/usage/m3.md`) |
| M4 mission branch, gate, fix loop | the window is 2026-10-25T18:19:15Z to 2026-10-29T22:05:24Z (4.1 days), of which 4 usage-limit gaps (1893 min) and 13 owner gaps (3067 min) are idle; the first M4 worker started 2026-10-29 09:25Z and the last implementation feature ended 21:10Z | 38 worker sessions (1662 turns, this integration excluded), scrutiny in three rounds (32, 3 and 2 review subagents) and user testing in two rounds, 48 validator sessions | 253.73 USD-eq in the window against the cap 200 (over); the orchestrator's own 111.86 USD-eq against 30 (over); 7.93 USD-eq per assertion fulfilled against M3's 4.75; mission to date 1260.74 USD-eq of the 1350 notify-and-ask line (`reports/usage/m4.md`, D-043) |

Token usage is pasted from the session sidecars into `reports/usage/<milestone>.md` at each
integration; the observatory's first job is to make that number visible.

## Licence

Apache-2.0, see [LICENSE](LICENSE).
