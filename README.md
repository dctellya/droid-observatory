# droid-observatory

A Rust desktop app that watches a Factory Mission build a business feature: which agent is on
what, whether each milestone is validated, where time and tokens go, and the one gate that
ships it. Lanes are the mission's orchestrated states; cards are milestones; the last lane is
the pull request into `main`.

**Status:** pre-mission. The repository holds the PRD, the rules, the CI gate and the
configuration contract. No application code yet: the walking skeleton is the mission's first
integration (ai/RULES.md rule 2). Rust targets in the `Makefile` activate the moment a
`Cargo.toml` lands.

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

## From a clean clone

Requirements: the Rust toolchain via [rustup](https://rustup.rs) (version pinned in
`rust-toolchain.toml`), and the tools `make doctor` lists.

```sh
git clone git@github.com:dctellya/droid-observatory.git
cd droid-observatory
make doctor      # which tools are missing and how to install them
make hooks       # enable the git hooks (once)
make check       # every light check the mission-branch CI runs
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
| `make fmt-check` / `make lint` / `make test` / `make deny` | rustfmt, clippy `-D warnings`, tests, cargo-deny | once `Cargo.toml` exists |
| `make replay-fixtures` | every `fixtures/streams/*.jsonl` through `observatory-cli replay` | once fixtures exist |
| `make app-dev` / `make app-build` | Tauri dev window / macOS bundle | once the app exists |

## Configuration

- `config/observatory.example.toml`: every runtime key with its default and reason. Copy to
  `~/.droid-observatory/config.toml`.
- `config/prices/<date>.toml`: the dated price table that turns tokens into money. Each cost on
  the board names the table it used.
- `.env.example`: environment variables, including the Grafana Cloud OTLP contract. Secrets go
  in the OS keychain on the desktop; the environment is the fallback and what CI uses.

## Running it as a mission

Models per role (ai/RULES.md rule 19, `.factory/settings.json`, D-006):
orchestrator `claude-fable-5-1`, workers `claude-sonnet-5-5`, validators `claude-opus-5-5`.

```sh
droid exec --mission --auto high \
  --model claude-fable-5-1 \
  --worker-model claude-sonnet-5-5 \
  --validator-model claude-opus-5-5 \
  -f SPEC.md
```

Reasoning effort per role is declared in `SPEC.md` §2. The mission works on
`mission/<name>`, one worktree per milestone, and ends in one pull request into `main`.

## The gate

Two workflows, one required check:

- `mission-branch.yml` on `mission/**`: docs, links, workflows, secrets, and (once code
  exists) fmt, clippy, tests, replay fixtures, cargo-deny. Aggregate job `mission-checks`.
  Budget: three minutes warm.
- `gate.yml` on pull requests to `main`: everything above plus Semgrep, Trivy, Checkov on the
  workflows, an SBOM, the macOS Tauri bundle with build provenance and cosign signatures.
  Aggregate job `gate`, the only check the `main` ruleset requires. It never deploys.

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
.github/       workflows (mission-branch, gate), CODEOWNERS, templates
.githooks/     pre-commit, commit-msg, pre-push (enable with `make hooks`)
config/        example config and dated price tables
docs/decisions/ one file per decision; README.md is generated
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

Token usage for these sessions is recorded by Factory, not by this repository yet; the
observatory's first job is to make that number visible.

## Licence

Apache-2.0, see [LICENSE](LICENSE).
