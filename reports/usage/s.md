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
