# droid-observatory — single source of commands.
# README, AGENTS.md, .githooks and .github/workflows all call these targets,
# so a command documented is a command CI runs (ai/RULES.md rule 5).
#
# Rust targets are no-ops when there is no Cargo.toml; they activate automatically.

SHELL := /bin/sh
.DEFAULT_GOAL := help

HAS_CARGO := $(wildcard Cargo.toml)
HAS_TAURI := $(wildcard apps/observatory-app/src-tauri/tauri.conf.json)
HAS_UI := $(wildcard apps/observatory-app/ui/Trunk.toml)
HAS_FIXTURES := $(wildcard fixtures/streams/*.jsonl)

# Thresholds (ai/RULES.md rule 4: every number has a home and a reason).
# MAX_FILE_KB: source files over this are a review smell; captured stream fixtures are exempt.
MAX_FILE_KB ?= 200
# COVERAGE_MIN: line-coverage floor for observatory-core (SPEC section 6). S reports it,
# M0 onward enforces 80; CI passes its own value through the same name.
COVERAGE_MIN ?= 80

# Gate cache (rule 27): check, test, lint and coverage run once per (code paths, toolchain, command) and replay
# the stored verdict after; FORCE=1 reruns. CI calls cargo directly and does not use the cache.
GATE := sh scripts/gate.sh $(if $(FORCE),--force)

.PHONY: help doctor hooks check check-docs check-links check-workflows check-file-size \
        check-all check-kit check-kit-lint check-contract check-contract-checks check-scripts check-gate-cache check-integrate check-integrate-long integrate integrate-long usage check-todos check-decisions-index decisions-index check-secrets redact-check check-wasm \
        fmt fmt-check lint test deny machete coverage replay-fixtures build ui-serve app-dev app-build e2e clean

help: ## List targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  %-24s %s\n", $$1, $$2}'

doctor: ## Check local tools and print install hints
	@sh scripts/doctor.sh

hooks: ## Install git hooks (one-time after clone)
	@git config core.hooksPath .githooks && echo "hooks: core.hooksPath=.githooks"

# --- Light checks: run on every mission-branch push, must stay under 3 minutes warm ---

check: ## All light checks (= mission-branch CI), cached by the code paths through scripts/gate.sh, then the contract checks (never cached: they read git history and GitHub)
	@$(GATE) check -- $(MAKE) --no-print-directory check-all
	@CONTRACT_REQUIRE_TESTLOG=1 $(MAKE) --no-print-directory check-contract

check-all: check-docs check-links check-workflows check-file-size check-todos check-decisions-index check-secrets redact-check check-wasm check-kit check-scripts fmt-check lint test deny machete replay-fixtures

check-scripts: check-gate-cache check-integrate check-integrate-long check-contract-checks check-kit-lint ## Shell tests of the repository scripts
	@sh scripts/tests/compile-share.sh
	@sh scripts/tests/usage-from-log.sh
	@sh scripts/tests/log-tool-call-duration.sh
	@sh scripts/tests/agents_md_test.sh
	@python3 scripts/tests/usage_test.py
	@python3 scripts/tests/usage_idle_test.py
	@python3 scripts/tests/usage_testing_test.py
	@python3 scripts/tests/usage_e2e_test.py
	@python3 scripts/tests/e2e_run_test.py
	@python3 scripts/tests/usage_rewrites_test.py
	@python3 scripts/tests/usage_ratios_test.py
	@python3 scripts/tests/usage_heartbeat_test.py
	@python3 scripts/tests/usage_clocks_test.py

usage: ## Write the tables of reports/usage/$(M).md between the usage markers (M=<milestone>, CAPS='orchestrator=30', OBSERVATORY_MISSION_DIR)
	@test -n "$(M)" || { echo "usage: make usage M=<milestone> [CAPS='role=USD ...'] [USAGE_ARGS='--from <ts> --to <ts>']" >&2; exit 2; }
	@python3 scripts/usage-report.py --milestone $(M) --cross-check $(foreach c,$(CAPS),--cap $(c)) $(USAGE_ARGS)

integrate: ## Land a milestone: no-ff merge, final gate, usage, staging files (M=<milestone> FROM='<branch>...'), see README 'Running a boundary'
	@test -n "$(M)" -a -n "$(FROM)" || { echo "usage: make integrate M=<milestone> FROM='<worktree-branch>...'" >&2; exit 2; }
	@sh scripts/integrate.sh "$(M)" $(FROM)

integrate-long: ## The integration's long work in parallel lanes: coverage, test timings, CI on the pushed head (M=<milestone>); one result file reports/integrate-long/<code tree>.json; start it as a background job
	@test -n "$(M)" || { echo "usage: make integrate-long M=<milestone> [FORCE=1]" >&2; exit 2; }
	@sh scripts/integrate-long.sh $(if $(FORCE),--force) "$(M)"

check-contract: ## The mechanical clauses of the validation contract: one line per check (scripts/contract-checks/README.md); GH_TOKEN enables the gh checks
	@sh scripts/contract-checks/run-all.sh

check-kit: ## UI-kit lint: mono and tabular figures, spacing overrides, recorded departures (scripts/check-kit.py, apps/observatory-app/kit-departures.md)
	@python3 scripts/check-kit.py

check-kit-lint: ## Self-test of the UI-kit lint: green on the tree, red on a planted override or sans cell, wiring
	@sh scripts/tests/kit_lint_test.sh

check-contract-checks: ## Self-test of the contract checks: green on the tree, red on a planted violation
	@sh scripts/tests/contract_checks_test.sh

check-integrate-long: ## Self-test of scripts/integrate-long.sh with a fake job set and a stub gh
	@sh scripts/tests/integrate_long_test.sh

check-integrate: ## Self-test of scripts/integrate.sh on throwaway repositories
	@sh scripts/tests/integrate_test.sh

check-gate-cache: ## Self-test of scripts/gate.sh: miss, hit, force, code-path key
	@sh scripts/tests/gate_test.sh

check-docs: ## Lint markdown
	@markdownlint-cli2 "**/*.md"

check-links: ## Check links in markdown (needs network)
	@lychee --config lychee.toml --no-progress "**/*.md"

check-workflows: ## Lint GitHub workflows
	@actionlint
	@sh scripts/check-ci-jobs.sh

check-file-size: ## Fail on tracked files larger than MAX_FILE_KB
	@MAX_FILE_KB=$(MAX_FILE_KB) sh scripts/check-file-size.sh

check-todos: ## Debt markers must reference an issue (#123) or a backlog item (B-123)
	@sh scripts/check-todos.sh

decisions-index: ## Regenerate docs/decisions/README.md
	@sh scripts/gen-decisions-index.sh > docs/decisions/README.md

check-decisions-index: ## Fail if the decisions index is stale
	@sh scripts/gen-decisions-index.sh | diff -u docs/decisions/README.md - \
	  || { echo "decisions index stale: run 'make decisions-index'"; exit 1; }

check-secrets: ## Scan the repository for committed secrets
	@gitleaks git --no-banner --redact .

redact-check: ## Fixtures are redacted: a second redaction pass changes nothing
	@sh scripts/check-redaction-idempotent.sh

# --- Rust (active once Cargo.toml exists) ---

# The host lint and test compile the UI crate for the host only; a break behind a wasm32 cfg (trunk's build) went unseen from
# 716f610 to d63f4cc (F-274). This is a check, no codegen.
check-wasm: ## The UI crate compiles for wasm32-unknown-unknown (cargo check, no codegen)
ifneq (,$(HAS_UI))
	@cargo check -p observatory-ui --target wasm32-unknown-unknown
else
	@echo "check-wasm: no UI yet, skipped"
endif

fmt: ## Format Rust code
ifneq (,$(HAS_CARGO))
	@cargo fmt --all
else
	@echo "fmt: no Cargo.toml yet, skipped"
endif

fmt-check: ## Check Rust formatting
ifneq (,$(HAS_CARGO))
	@cargo fmt --all -- --check
else
	@echo "fmt-check: no Cargo.toml yet, skipped"
endif

lint: ## Clippy with warnings denied
ifneq (,$(HAS_CARGO))
	@$(GATE) lint -- cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cognitive_complexity
else
	@echo "lint: no Cargo.toml yet, skipped"
endif

test: ## Run the test suite
ifneq (,$(HAS_CARGO))
	@$(GATE) test -- cargo test --workspace --all-features
else
	@echo "test: no Cargo.toml yet, skipped"
endif

deny: ## cargo-deny advisories, licences, bans, sources
ifneq (,$(HAS_CARGO))
	@cargo deny check
else
	@echo "deny: no Cargo.toml yet, skipped"
endif

machete: ## Fail on unused dependencies
ifneq (,$(HAS_CARGO))
	@cargo machete
else
	@echo "machete: no Cargo.toml yet, skipped"
endif

coverage: ## Line coverage of observatory-core (run alone, not part of check)
ifneq (,$(HAS_CARGO))
	@$(GATE) coverage -- cargo llvm-cov -p observatory-core --fail-under-lines $(COVERAGE_MIN)
else
	@echo "coverage: no Cargo.toml yet, skipped"
endif

replay-fixtures: ## Replay every captured stream fixture through observatory-cli
ifneq (,$(HAS_FIXTURES))
	@for f in fixtures/streams/*.jsonl; do \
	  case "$$f" in *.usage-log.jsonl) continue;; esac; \
	  echo "replay $$f"; cargo run -q -p observatory-cli -- replay "$$f" >/dev/null || exit 1; \
	done
else
	@echo "replay-fixtures: no fixtures/streams/*.jsonl yet, skipped"
endif

build: ## Build all crates (release)
ifneq (,$(HAS_CARGO))
	@cargo build --workspace --release
else
	@echo "build: no Cargo.toml yet, skipped"
endif

ui-serve: ## Serve the Leptos UI in the browser on the dev port
ifneq (,$(HAS_UI))
	@trunk serve --config apps/observatory-app/ui/Trunk.toml --port 3141 --open false
else
	@echo "ui-serve: no UI yet, skipped"
endif

app-dev: ## Open the Tauri dev window
ifneq (,$(HAS_TAURI))
	@cargo tauri dev
else
	@echo "app-dev: no Tauri app yet, skipped"
endif

app-build: ## Build the Tauri desktop bundle (macOS)
ifneq (,$(HAS_TAURI))
	@cargo tauri build $(if $(BUNDLES),--bundles $(BUNDLES))
else
	@echo "app-build: no Tauri app yet, skipped"
endif

# The window suite (owner item 14): the debug app with the embedded WebDriver server (`e2e` cargo
# feature, never a release build), driven by WebdriverIO through scripts/e2e_run.py: a failing first attempt keeps its evidence in
# reports/e2e/<E2E_DATE>/<E2E_STAMP>/first-failure/ and reruns the failed specs once; the outcome (passed, flaked, failed) is in
# record.json. JUnit and screenshots land in the same run directory, which no later run deletes (a flake stays countable by the
# usage report); reports/e2e/latest is a symlink to the newest run. The app's store, spool and fake Droid live in a scratch directory.
E2E_DATE ?= $(shell date +%Y-%m-%d)
E2E_STAMP ?= $(shell date +%H%M%S)
E2E_RUN_DIR ?= $(or $(TMPDIR),/tmp)/observatory-e2e
e2e: ## Build the app with --features e2e and run the WebdriverIO suite (reports/e2e/<date>/<HHMMSS>/)
ifneq (,$(HAS_TAURI))
	@cargo build -p observatory-cli
	@cargo tauri build --debug --no-bundle --features e2e
	@rm -rf "$(E2E_RUN_DIR)"
	@cd apps/observatory-app/e2e && npm ci
	@E2E_DATE=$(E2E_DATE) python3 scripts/e2e_run.py run --report-dir "reports/e2e/$(E2E_DATE)/$(E2E_STAMP)" --latest "reports/e2e/latest" --run-dir "$(E2E_RUN_DIR)" --cwd apps/observatory-app/e2e -- npx wdio run wdio.conf.ts
else
	@echo "e2e: no Tauri app yet, skipped"
endif

clean: ## Remove build output
	@rm -rf target dist
