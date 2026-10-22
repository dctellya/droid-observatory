#!/bin/sh
# Reports which tools the Makefile needs and how to install the missing ones.
# Exit 0 always: this is advisory. CI installs its own tools.
set -u

check() {
  name=$1; hint=$2
  if command -v "$name" >/dev/null 2>&1; then
    printf '  ok       %s\n' "$name"
  else
    printf '  missing  %-18s %s\n' "$name" "$hint"
  fi
}

echo "required for 'make check':"
check rustup           "https://rustup.rs (toolchain pinned in rust-toolchain.toml)"
check cargo            "installed by rustup"
check cargo-deny       "cargo install cargo-deny --locked"
check cargo-machete    "cargo install cargo-machete --locked"
check gitleaks         "brew install gitleaks"
check markdownlint-cli2 "npm i -g markdownlint-cli2  (or brew install markdownlint-cli2)"
check lychee           "brew install lychee  (or cargo install lychee --locked)"
check actionlint       "brew install actionlint"
check jq               "brew install jq  (used by .factory hooks)"
echo
echo "required for the desktop app:"
check trunk            "cargo install trunk --locked"
check cargo-tauri      "cargo install tauri-cli --version ^2 --locked"
echo
echo "required for 'make coverage' (not part of 'make check'):"
check cargo-llvm-cov   "cargo install cargo-llvm-cov --locked  (and: rustup component add llvm-tools-preview)"
echo
echo "run 'make hooks' once after cloning to enable git hooks."
