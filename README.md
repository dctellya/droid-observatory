# droid-observatory

A Rust desktop app that watches a Factory Mission build a business feature: which agent is on
what, whether each milestone is validated, where time and tokens go, and the one gate that
ships it. It is a mission watcher: it measures the waste and effectiveness (cycle time, cost)
of the engineering cycle from the spec to the pull request's checks going green. Lanes are the
mission's seven phases; cards are milestones; roles are colour; the human's interventions are
overlays; time, waste, cost and model drift are the Panel. What happens after the merge
(release, observe, deploys, flags) belongs to the delivery platform and is out of scope: the
last event the app records is `handed_to_cd`.

**Status:** mission in progress. The walking skeleton (S), the stream capture and mapping milestone (M0) and the
board in replay and attach (M1, ai/RULES.md rule 2, SPEC §3) are integrated on `mission/observatory-v1`: a Cargo
workspace, the canonical `Event`, mapping v1 with its golden file (seven phase lanes, D-024), a SQLite pointer
store, `observatory-cli replay` and `inspect`, view models, the attach watcher, the usage-log source (D-029), a
Leptos board with Details and agents views and a Tauri host that attaches to live sessions. The Leptos go/no-go at
M1 is **go** (D-030). Dogfooding started at the M1 integration: `make app-dev` from this checkout lists this
mission's own orchestrator session on the board (`reports/usage/m1.md`, session log entry 35). Host, intervention,
gate and metrics follow in M2-M5 (SPEC §3).

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

Human interventions (and, later, a commander's actions) are overlay markers on these lanes. Two
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
- `.env.example`: environment variables, including the Grafana Cloud OTLP contract. Secrets go
  in the OS keychain on the desktop; the environment is the fallback and what CI uses.

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

Grafana Cloud over OTLP is the telemetry backend (D-004): the app's own traces and metrics,
and the Panel's mission series (`observatory.*`). Off by default; the app works fully offline.
Endpoint and credentials come from `OTEL_EXPORTER_OTLP_*` or the keychain, never from the
repository.

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

Token usage is pasted from the session sidecars into `reports/usage/<milestone>.md` at each
integration; the observatory's first job is to make that number visible.

## Licence

Apache-2.0, see [LICENSE](LICENSE).
