# Usage: milestone S (walking skeleton)

Source: the sidecars `~/.factory/sessions/-Users-dct-code-EllyaOrg-projects-dalimaray-droid-observatory/<id>.settings.json`
(`tokenUsage` per session, children from `childInclusiveTokenUsageBySessionId`), read with `jq` on
2026-10-23 at the S integration (SPEC §2, rule 7). Priced with `config/prices/2026-10-22.toml`
(`verified = true`): cost = input x input price + output x output price + cache read x cache read
price + cache write x cache write price, per million tokens, by the session's own `model`. Thinking
tokens are listed and not priced (the table has no thinking price). `factoryCredits` is shown as the
sidecar reports it; its unit is not documented in the file.

Cap: 40 USD-eq (SPEC §2). Reasoning effort of every priced session: `high`.

## Result

| Part | Sessions | USD-eq |
|---|---|---|
| Workers, `claude-sonnet-5-5` | 6 | 4.49 |
| Validators, `claude-opus-5-5` (scrutiny and user testing, with their children) | 11 | 8.72 |
| Validator child on `kimi-k3` | 1 | not priced (no price row) |
| **S, priced** | 17 | **13.21 of 40 (33.0 %)** |
| Orchestrator `claude-fable-5.1`, session `e194b9e3` | 1 | not attributable to S, see below |

The orchestrator session is a single session for the whole mission (planning, contract work,
readiness, S). Its sidecar is cumulative and the repository holds no snapshot at the start of S, so
the share spent on S is **not exposed**. Cumulative own usage of `e194b9e3` when read at the S
integration, as a baseline for the M0 delta: input 629 245, output 499 465, cache read 32 674 587,
cache write 3 520 336, thinking 114 552, factoryCredits 31 002 862, which prices at 83.44 USD-eq at
the `claude-fable-5.1` row. That number is mission-to-date, not S, and is already above the S cap
on its own; the M0 re-cap proposal has to decide how the orchestrator is capped.

## Sessions

| Session | Feature or role | Model | Input | Output | Cache read | Cache write | Thinking | Credits | USD-eq |
|---|---|---|---|---|---|---|---|---|---|
| `ca58108e` | s-workspace-scaffold | claude-sonnet-5-5 | 1 583 | 42 900 | 4 777 290 | 155 621 | 12 180 | 518 968 | 1.30 |
| `73cd8242` | s-fixture-snapshot | claude-sonnet-5-5 | 1 220 | 17 941 | 1 238 224 | 66 254 | 2 781 | 188 042 | 0.47 |
| `b8c3d386` | s-core-event-model | claude-sonnet-5-5 | 1 157 | 26 808 | 2 006 951 | 89 249 | 5 688 | 277 243 | 0.69 |
| `72e5184a` | s-cli-replay | claude-sonnet-5-5 | 1 293 | 40 417 | 2 670 858 | 131 226 | 11 838 | 400 258 | 1.00 |
| `bfc3f3f7` | s-app-empty-lanes | claude-sonnet-5-5 | 1 278 | 14 362 | 1 427 943 | 129 299 | 1 655 | 244 387 | 0.61 |
| `57905379` | s-ci-wiring | claude-sonnet-5-5 | 1 175 | 12 336 | 1 012 867 | 74 249 | 2 712 | 164 583 | 0.41 |
| `6eb88099` | scrutiny-validator-s | claude-opus-5-5 | 827 | 19 667 | 684 083 | 102 479 | 2 260 | 417 330 | 1.05 |
| `35e4968d` | scrutiny reviewer (child) | claude-opus-5-5 | 16 | 8 675 | 249 775 | 56 735 | 1 059 | 202 881 | 0.51 |
| `3e7bc50a` | scrutiny reviewer (child) | claude-opus-5-5 | 22 | 9 082 | 440 817 | 62 506 | 2 161 | 232 973 | 0.58 |
| `ee0c614a` | scrutiny reviewer (child) | claude-opus-5-5 | 24 | 11 752 | 577 163 | 82 305 | 3 067 | 304 843 | 0.76 |
| `571a9e79` | scrutiny reviewer (child) | claude-opus-5-5 | 18 | 12 566 | 403 976 | 61 909 | 3 875 | 256 696 | 0.64 |
| `e78e6d4f` | scrutiny reviewer (child) | claude-opus-5-5 | 22 | 12 974 | 397 505 | 60 057 | 3 437 | 255 746 | 0.64 |
| `5f9d68a5` | scrutiny reviewer (child) | claude-opus-5-5 | 24 | 13 937 | 445 230 | 53 834 | 2 265 | 254 826 | 0.64 |
| `d0304e2e` | user-testing-validator-s | claude-opus-5-5 | 855 | 19 420 | 924 229 | 249 370 | 3 216 | 728 369 | 1.82 |
| `676e401d` | user-testing flow validator (child) | claude-opus-5-5 | 14 | 10 704 | 245 988 | 56 952 | 371 | 219 240 | 0.55 |
| `cedd249c` | user-testing flow validator (child) | claude-opus-5-5 | 36 | 15 667 | 792 962 | 62 767 | 1 629 | 314 372 | 0.79 |
| `cf2e2456` | user-testing flow validator (child) | claude-opus-5-5 | 28 | 17 218 | 587 123 | 57 174 | 2 112 | 299 113 | 0.75 |
| `49847d4f` | user-testing flow validator (child) | **kimi-k3** | 109 901 | 6 499 | 227 960 | 0 | 0 | 198 239 | no price row |

Subtotals: workers 7 706 input, 154 764 output, 13 134 133 cache read, 645 898 cache write,
36 854 thinking, 1 793 481 credits. Validators on `claude-opus-5-5`: 1 886 input, 151 662 output,
5 748 851 cache read, 906 088 cache write, 25 452 thinking, 3 486 389 credits.

## Not in the table

- `acd7d48a` (user-testing flow validator "S CLI and cargo tests", `kimi-k3`): its `session_start`
  names `d0304e2e` as the caller, but `d0304e2e` does not list it among its children. Own usage:
  input 55 397, output 8 513, cache read 434 018, thinking 0, credits 169 642. No price row.
- The model of the resumed turns of `d0304e2e` (resumed 2026-10-23T06:40Z) is not recorded in the
  file; the sidecar `model` field reads `claude-opus-5-5`.
- The first start of `m0-integrate-s`, session `82a61cf6`, ran on `kimi-k3` from 2026-10-23T06:40:42Z
  to 07:13:30Z and made no commit. Own usage: input 75 561, output 6 614, cache read 247 773,
  thinking 0, credits 158 830. It belongs to the M0 boundary, not to S, and has no price row.
  See `docs/decisions/D-018-mission-models-are-invariant.md`.
- The second start, `2c04bd42` (`claude-sonnet-5-5`, this boundary), is open when this file is
  written and goes to `reports/usage/m0.md`.

## Wall time

Worker sessions of S, first to last timestamp of each transcript: `ca58108e` started
2026-10-22T16:49:27Z; `57905379` ended 17:32:47Z; scrutiny `6eb88099` ended 17:39:42Z. User testing
`d0304e2e` started 17:45:31Z and its first run ended 2026-10-23T06:40:02Z (it waited on a usage
limit overnight, then resumed). `assistantActiveTimeMs` summed over the eight parent sessions:
3 836 981 ms (63.9 min).

## Screenshot

The owner adds `reports/screenshots/2026-10-23-s.png` (display unlocked, `make app-dev` window with
the nine empty lanes) at this boundary. Byte size recorded when the file arrives.

## Addendum: round-2 user testing on `claude-opus-5-5` (read 2026-10-23, second integration pass)

Round 2 re-ran VAL-SKEL-004..009 and VAL-SKEL-020 on `claude-opus-5-5` after D-018 (round 1 had run
them on `kimi-k3`). Same source and pricing as above (Opus row: 4.0 input, 20.0 output, 0.20 cache
read, 5.0 cache write per million). All three sessions: `claude-opus-5-5`, effort `high`.

| Session | Role | Input | Output | Cache read | Cache write | Thinking | Credits | USD-eq |
|---|---|---|---|---|---|---|---|---|
| `94334a8c` | user-testing-validator-s, round 2 | 1 086 | 14 868 | 656 708 | 162 139 | 1 693 | 496 173 | 1.24 |
| `067dcdb4` | flow validator `cli-tests-r2` (child) | 28 | 19 209 | 612 342 | 78 584 | 2 523 | 359 878 | 0.90 |
| `c9f10788` | flow validator `clean-clone-r2` (child) | 26 | 9 118 | 358 599 | 57 628 | 1 016 | 216 934 | 0.54 |
| **Round 2** | 3 sessions | 1 140 | 43 195 | 1 627 649 | 298 351 | 5 232 | 1 073 985 | **2.69** |

The "S, priced" total of 13.21 above is **unchanged** (13.21 of 40, 33.0 %). With round 2 added,
the S milestone's validators and workers come to 15.90 USD-eq (39.8 % of the cap). The round-1
`kimi-k3` sessions stay listed unpriced above; the round-2 sessions replace their evidence.

## Orchestrator delta since the baseline (session `e194b9e3`, `claude-fable-5.1`)

Owner decision 13: the orchestrator is capped at 20 % of each milestone cap, measured as the delta of
its own `tokenUsage` from the baseline above. Read from the sidecar at 2026-10-23T09:07Z (the session
is still running, so the number grows):

| | Input | Output | Cache read | Cache write | Thinking | Credits | USD-eq |
|---|---|---|---|---|---|---|---|
| Baseline (S integration, first pass) | 629 245 | 499 465 | 32 674 587 | 3 520 336 | 114 552 | 31 002 862 | 83.44 |
| Read now (absolute) | 629 309 | 537 587 | 37 126 596 | 4 794 045 | 126 147 | 38 579 320 | 102.38 |
| **Delta** | 64 | 38 122 | 4 452 009 | 1 273 709 | 11 595 | 7 576 458 | **18.94** |

Fable row: 10.0 input, 50.0 output, 0.25 cache read, 12.5 cache write per million. The M0 orchestrator
cap is 12 USD-eq (SPEC §2); the delta so far is 18.94, which is 158 % of it, and includes the S to M0
boundary work (model-fallback episode, restarts, owner decisions) before any M0 feature ran. The new
absolute `tokenUsage` above becomes the baseline for M1 only after the M0 boundary re-reads it.

Two orchestrator child sessions on `claude-fable-5.1` at effort `medium` (`180b4a60`, `4162070f`; tag
`subagent`) are listed in the orchestrator's children; their own usage is 19 743 and 16 529 output
tokens, and they belong to the M0 boundary, not to S. The `high` effort of the orchestrator row does not
apply to them.

## Screenshot request

`reports/screenshots/2026-10-23-s.png`: the owner runs `make app-dev` from the mission branch with the
display unlocked and adds the file; the byte size is recorded here when it arrives.
