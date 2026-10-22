# D-002: Tauri 2 + Leptos, with an explicit go/no-go at M1

**Date:** 2026-10-22
**Status:** accepted (provisional until M1)

## Context

Constraint: Rust. Preference: Tauri 2. The hard part is `observatory-core` (ingest,
mapping, metrics), not the UI. Rich content (markdown findings, QA video, PR rendering)
needs a webview.

## Decision

Tauri 2 shell with a Leptos (CSR, Trunk) frontend; core logic in a UI-free crate behind
Tauri commands with serde types shared end to end.

Go/no-go at M1 completion: the board renders the M0 replay (nine lanes, correct card
positions, Details timeline) with at most 3 worker sessions of rework after the first
scrutiny pass, and the Trunk/WASM build succeeds from a clean checkout with the documented
command. If either fails, the frontend switches to Tauri 2 + TypeScript (Svelte) from M2,
`observatory-core` unchanged.

## Alternatives

Tauri 2 + Dioxus (same risks, blurs "why Tauri"); Tauri 2 + TypeScript (two languages,
richest UI ecosystem: the fallback); iced and egui (weak for text-heavy Details).

## Consequences

Named risks: less Leptos in Droid training data than React/Svelte; the WASM/Trunk toolchain
inside a mission is a new failure mode. `rust-toolchain.toml` includes
`wasm32-unknown-unknown`; the gate's `bundle` job installs `trunk` and `tauri-cli`.
