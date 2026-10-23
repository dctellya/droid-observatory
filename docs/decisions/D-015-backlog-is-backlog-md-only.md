# D-015: The backlog is BACKLOG.md only; the hybrid backlog is withdrawn

**Date:** 2026-10-23
**Status:** accepted (owner comment on the mission proposal, 2026-10-22, entry 27)

## Context

SPEC §8 planned a hybrid backlog: GitHub issues labelled `backlog` created at S integration, with
`scripts/gen-backlog.sh` regenerating `BACKLOG.md` from them. That reopened the Issues question
the owner had closed the day before: deferred work goes to `BACKLOG.md`, not to GitHub issues
(AGENTS.md, rule 14 as the owner reads it). The owner asked to pick `BACKLOG.md` only, "unless you
want Issues back on purpose" (entry 27, comment 4).

## Decision

`BACKLOG.md` is the only backlog. No issue is created, no `backlog` label exists, and
`scripts/gen-backlog.sh` is not written. Deferred items are `B-NNN` rows with a reason and an
"earliest" column; settled rows are marked done with the date, dropped rows are struck through.
The integration feature writes new rows from the workers' handoffs. SPEC §8 is amended as-built.

## Alternatives

Hybrid backlog (two sources of truth and an issue-creating mission the owner did not ask for);
issues only (loses the offline, reviewable-in-PR history that `BACKLOG.md` gives).

## Consequences

`ai/RULES.md` rule 14 still reads "GitHub issues labelled `backlog`, indexed in `BACKLOG.md`".
The file is the owner's; the proposed wording is "everything else goes to `BACKLOG.md` with its
reason", and its change is on the S to M0 boundary list.
