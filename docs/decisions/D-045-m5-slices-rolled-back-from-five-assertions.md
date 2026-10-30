# D-045: The five-assertion slice cap is rolled back as its inverse; M5 slices by M4-measured turns per assertion, in batches

**Date:** 2026-10-29
**Status:** accepted; owner inbox 2026-10-29-1915 ("this should be applied ASAP"), relayed by pre-roci; inverse of F-158 (owner item 21 of the M3 boundary)

## Context

Item 21 of the M3 boundary (F-158) cut every slice to five assertions or fewer, at 23.3 worker turns per assertion. The owner reads
it at the coordination ratio as worse: a worker finishes a slice for about 1.4 USD-eq and the officer lands it for 1.55 to 2.81, so
coordination ran at 1.3 to 2 against the work. M4 confirms it in the record: 10.3 orchestrator turns per feature boundary against the
target 3, 111.86 USD-eq of orchestrator cost against 30. Rule 29's own test: a lever that measures worse is rolled back by its
inverse at the next boundary.

## Decision

The owner's rule, applied as written:

1. M5 slices at eight to ten assertions each, with the turn budget as the guard (R-054): `turnBudget` = assertions x M4-measured
   turns per assertion by kind. A slice over 120 planned turns is split; a slice under 60 is merged with its neighbour when they share
   no file conflicts.
2. Batches: independent slices (no shared files, no prerequisite) are dispatched together so the orchestrator wakes per batch
   (`returnToOrchestrator` false between them). Target coordination ratio under 1.0.
3. Window: the M5 boundary reads the coordination ratio; if it is not under 1.3 the re-cut is itself reopened.
4. Everything else in the M5 plan is unchanged until the owner's M4 feedback arrives.

**M4's turns per assertion (the measurement item 1 asks for).** `reports/usage/m4.md` has no per-kind figure (its Feature size
table lists turns per feature, its Cost per assertion table lists cost); the M4 overall mean is used and stated, with the figures
derived from the same report and `features.json`:

| Basis | Worker turns | Assertions | Turns per assertion |
|---|---|---|---|
| M4 overall mean: every worker feature session (core, ui, infra, 39 features) | 1525 | 32 fulfilled | 47.7 |
| core-worker and ui-worker features only (contract work; the 21 infra features are the M3 boundary's tooling, 702 turns) | 823 | 32 fulfilled | 25.7 |
| core-worker features / ui-worker features (410 / 413 turns) | 410 / 413 | 12 / 12 attributed in `features.json` | 34.2 / 34.4 |

The per-kind attribution in `features.json` is partial (most M4 core features keep `fulfills` empty; the walk features carry
them), so the third row is an upper bound; the second row is the figure used for the guard: **25.7 turns per assertion**
(M1 20.3, M2 29.9, M3 23.3).

**Result of the guard.** At 25.7 turns per assertion, 120 turns is 4.7 assertions; a slice of eight to ten assertions plans 206 to 257
turns and is over 120, so rule 1 splits it again. The two clauses of the owner's rule (eight to ten assertions; under 120 planned
turns) cannot both hold on M4's measurement. The eight planned M5 slices (4 to 5 assertions, 103 to 128 turns) already sit at the
guard. The re-cut therefore keeps the eight slices and moves the saving to the second clause, batches (the order and batches are
in the boundary report; the budgets are 5 x 25.7 = 128 capped at 120 and 4 x 25.7 = 103). The conflict is raised to the owner in
the boundary report; the guard (R-054, the 120-turn cap, rule 29) is kept as the binding clause until the owner says otherwise.

## Alternatives

- **Slices of eight to ten assertions at 206 to 257 planned turns:** over the 120-turn cap rule 29 sets; M3's two features over 120
  turns (136, 146) are the receipt for why that cap exists.
- **Keep the five-assertion cap:** the owner's roll-back; coordination cost is what it measured.
- **Use the 47.7 mean as the factor:** it includes 21 tooling features that claim few assertions; it would split every slice to two.

## Consequences

- `features.json` is edited by the orchestrator (the integrator does not touch the mission directory's plan): `turnBudget` per M5
  slice = assertions x 25.7, capped at 120; batches named in the boundary report.
- The M5 boundary reads the coordination ratio (orchestrator turns per slice boundary against worker turns) and reopens the re-cut
  if it is not under 1.3.
- F-158 is superseded by this decision; F-138 (rule 29) stands.
