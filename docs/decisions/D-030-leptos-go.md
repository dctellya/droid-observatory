# D-030: Leptos stays: go at the M1 go/no-go

**Date:** 2026-10-24
**Status:** accepted (resolves the go/no-go of [D-002](D-002-tauri-leptos-go-no-go.md) at the M1 integration, merge commit `c2fb42acf97883c1e1aa0458ebbc26fbc938bad2`); go, the frontend stays Tauri 2 + Leptos (CSR, Trunk)

## Context

D-002 set two conditions at M1 completion: (1) the board renders the M0 replay with at most 3 worker
sessions of rework after the first scrutiny pass, and (2) the Trunk/WASM build succeeds from a clean
checkout with the documented command. A failure of either switches the frontend to Tauri 2 + TypeScript
(Svelte) from M2, `observatory-core` unchanged. The board now has seven phase lanes (D-024), not the nine
lanes D-002 names; the condition is read against the seven.

## Evidence

- **Rework sessions: 1 (limit 3).** UI rework counted as worker sessions that fixed Leptos/WASM code after the
  first scrutiny pass or its own validation: `m1-ui-board-views` needed one session of compile fixes (handoff
  `3211ca66`); `m1-ui-details-and-agents` 0 (handoff `6c647781`); `m1-tauri-attach` 0 (handoff `39629802`). Counted
  by raw number, the worker sessions started after the first scrutiny pass (`7813bb96`, 2026-10-24T00:03:03Z)
  are `ab5836cf` (`m1-fix-turn-usage-panel-only`, touches the UI panel and core), `40a42973`
  (`m1-fix-session-name-user-tag`) and `443ef46e` (`m1-fix-session-name-everywhere`), the last two in core only:
  at most 3, so the limit holds under either reading. One mechanical orchestrator fix (`f1b6b45`, serde and
  serde_json removed from `src-tauri`) is not a worker session.
- **Clean-checkout build with the documented command.** `cd apps/observatory-app && trunk build --config
  ui/Trunk.toml` (README "The board (UI)"): clean clone 56.95 s (`39629802`) and 61.04 s (user-testing
  validator `3c9765d3`, VAL-BOARD-035), exit 0. Re-run at this integration in a fresh clone of `mission/observatory-v1` at
  `add7b7c` with the README command and the shared (warm) `CARGO_TARGET_DIR`: `real 0m11.426s`, `✅ success`. Cold build in the worktree 33.18 s and warm 1.894 s
  (`3211ca66`), warm 2.139 s (`6c647781`), 10.3 s incremental after a core change (`39629802`).
- **The board renders the M0 replay.** Seven lanes, cards, Details timeline and the agents view were
  validated in the browser path on port 3141 (VAL-BOARD-004, -005, -019 passed in user-testing-m1) and the
  Tauri host attaches to live sessions (`m1-tauri-attach`; dogfooding, `reports/usage/m1.md` and
  `ai/session/2026-10-22-session.md` entry 35).

## Decision

Go. The frontend stays Tauri 2 + Leptos for M2 to M5. The TypeScript fallback is not taken.

## Alternatives considered

- Tauri 2 + TypeScript (Svelte): rejected, both D-002 conditions hold; the switch would cost the three UI
  features already built and add a second language.
- Tauri 2 + Dioxus, iced, egui: rejected in D-002 and nothing in the M1 evidence reopens them.

## Consequences

- Named risks of D-002 remain: little Leptos in training data and the WASM/Trunk toolchain. The clean-clone
  build costs about a minute; `make check` does not build the UI (the gate's `bundle` job does).
- Plan-lane overflow and the session naming question are UI items for M2 (BACKLOG B-041, B-046).
