#!/bin/sh
# Devcontainer provisioning (BACKLOG B-001): Linux parity for `make check` and the
# core crates. The Tauri window itself needs macOS (D-002); the Linux webkit
# packages below let `cargo check` cover apps/observatory-app.
# Versions verified on 2026-10-22 against each project's latest GitHub release
# (ai/RULES.md rule 3). Renovate tracks them through the comments on each line.
set -eu

ACTIONLINT_VERSION=1.7.12 # renovate: datasource=github-releases depName=rhysd/actionlint
LYCHEE_VERSION=0.24.2 # renovate: datasource=github-releases depName=lycheeverse/lychee extractVersion=^lychee-v(?<version>.*)$
GITLEAKS_VERSION=8.30.1 # renovate: datasource=github-releases depName=gitleaks/gitleaks
CARGO_DENY_VERSION=0.20.2 # renovate: datasource=github-releases depName=EmbarkStudios/cargo-deny
MARKDOWNLINT_CLI2_VERSION=0.24.0 # renovate: datasource=npm depName=markdownlint-cli2

arch=$(uname -m)
case "$arch" in
  x86_64)  go_arch=amd64; rust_arch=x86_64;  gl_arch=x64 ;;
  aarch64) go_arch=arm64; rust_arch=aarch64; gl_arch=arm64 ;;
  *) echo "unsupported architecture: $arch"; exit 1 ;;
esac

bin=/usr/local/bin
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

echo "== apt: Tauri 2 build prerequisites (Debian) and jq"
sudo apt-get update -qq
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
  jq libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev
sudo rm -rf /var/lib/apt/lists/*

echo "== rust toolchain from rust-toolchain.toml"
rustup show active-toolchain

echo "== actionlint ${ACTIONLINT_VERSION}"
curl -fsSL "https://github.com/rhysd/actionlint/releases/download/v${ACTIONLINT_VERSION}/actionlint_${ACTIONLINT_VERSION}_linux_${go_arch}.tar.gz" \
  | sudo tar -xz -C "$bin" actionlint

echo "== lychee ${LYCHEE_VERSION}"
curl -fsSL "https://github.com/lycheeverse/lychee/releases/download/lychee-v${LYCHEE_VERSION}/lychee-${rust_arch}-unknown-linux-gnu.tar.gz" \
  | sudo tar -xz -C "$bin" lychee

echo "== gitleaks ${GITLEAKS_VERSION}"
curl -fsSL "https://github.com/gitleaks/gitleaks/releases/download/v${GITLEAKS_VERSION}/gitleaks_${GITLEAKS_VERSION}_linux_${gl_arch}.tar.gz" \
  | sudo tar -xz -C "$bin" gitleaks

echo "== cargo-deny ${CARGO_DENY_VERSION}"
curl -fsSL "https://github.com/EmbarkStudios/cargo-deny/releases/download/${CARGO_DENY_VERSION}/cargo-deny-${CARGO_DENY_VERSION}-${rust_arch}-unknown-linux-musl.tar.gz" \
  | tar -xz -C "$tmp"
sudo install -m 0755 "$tmp"/cargo-deny-*/cargo-deny "$bin/cargo-deny"

echo "== markdownlint-cli2 ${MARKDOWNLINT_CLI2_VERSION}"
npm install -g --no-fund --no-audit "markdownlint-cli2@${MARKDOWNLINT_CLI2_VERSION}"

echo "== git hooks"
make hooks

echo "== doctor"
make doctor
