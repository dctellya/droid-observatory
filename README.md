# droid-observatory

A Rust desktop app that watches a Factory Mission build a business feature: which agent is on
what, whether each milestone is validated, where time and tokens go, and the one gate that
ships it. Lanes are the mission's orchestrated states; cards are milestones; the last lane is
the pull request into `main`.

**Status:** mission in progress. The walking skeleton (S) and the stream capture and mapping
milestone (M0, ai/RULES.md rule 2, SPEC §3) are integrated on `mission/observatory-v1`: a Cargo
workspace, the canonical `Event`, mapping v1 with its golden file, a SQLite pointer store,
`observatory-cli replay` and `inspect` over the redacted M0 capture, and a window with nine empty
lanes. Board, host, intervention, gate and metrics follow in M1-M5 (SPEC §3).

## What it will show

| Lane | Holds |
|---|---|
| 1 Intent | prompt, spec, acceptance criteria, re-prompts |
| 2 Orchestrator | plan, milestones, task assignment, replans |
| 3 Workers | one sub-lane per worker: files, tests, commits |
| 4 Validators, scrutiny | findings, blocking vs non-blocking, rework |
| 5 Validators, user testing | QA scenarios against the built thing |
| 6 Integration | worktree landings on `mission/<name>`, fast checks |
| 7 Human | approvals, answers, the review, the merge |
| 8 Outside | the PR, CI, attestations, deploys |
| 9 Time, waste, cost, models | tokens per role, waste by cause, budget |

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
bypass, zero required approvals (the human in lane 7 merges by hand, the gate is the review).

## Observability

Grafana Cloud over OTLP is the telemetry backend (D-004): the app's own traces and metrics,
and the lane-9 mission series (`observatory.*`). Off by default; the app works fully offline.
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

Token usage is pasted from the session sidecars into `reports/usage/<milestone>.md` at each
integration; the observatory's first job is to make that number visible.

## Licence

Apache-2.0, see [LICENSE](LICENSE).
