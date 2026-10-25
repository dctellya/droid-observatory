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

# Gate cache (rule 27): check, test, lint and coverage run once per (tree, toolchain, command) and replay
# the stored verdict after; FORCE=1 reruns. CI calls cargo directly and does not use the cache.
GATE := sh scripts/gate.sh $(if $(FORCE),--force)

.PHONY: help doctor hooks check check-docs check-links check-workflows check-file-size \
        check-all check-scripts check-gate-cache check-todos check-decisions-index decisions-index check-secrets redact-check \
        fmt fmt-check lint test deny machete coverage replay-fixtures build ui-serve app-dev app-build clean

help: ## List targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  %-24s %s\n", $$1, $$2}'

doctor: ## Check local tools and print install hints
	@sh scripts/doctor.sh

hooks: ## Install git hooks (one-time after clone)
	@git config core.hooksPath .githooks && echo "hooks: core.hooksPath=.githooks"

# --- Light checks: run on every mission-branch push, must stay under 3 minutes warm ---

check: ## All light checks (= mission-branch CI), cached by tree through scripts/gate.sh
	@$(GATE) check -- $(MAKE) --no-print-directory check-all

check-all: check-docs check-links check-workflows check-file-size check-todos check-decisions-index check-secrets redact-check check-scripts fmt-check lint test deny machete replay-fixtures

check-scripts: check-gate-cache ## Shell tests of the repository scripts
	@sh scripts/tests/compile-share.sh
	@sh scripts/tests/usage-from-log.sh
	@sh scripts/tests/log-tool-call-duration.sh

check-gate-cache: ## Self-test of scripts/gate.sh: miss, hit, force, key changes
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

clean: ## Remove build output
	@rm -rf target dist
