# Spec template

The structure every SPEC.md follows in this project. Adapted from the artstudio template for a build run as a Factory Mission: milestones instead of team phases, one mission branch, one pull request.

```
# <product> — Spec for the mission
Owner: David Carbajosa (architect, reviewer, the human in lane 7). Implementers: one Factory Mission (orchestrator, workers, validators) under ai/RULES.md. Status: v<X.Y> (<date>). Living document: §9 is the human feedback channel, §10 the as-built history.

## 1. Purpose            what it is, for whom, what it must prove, what is out of scope (explicitly); the PRD it implements and the review notes it answers
## 2. Mission setup      the mission command with its flags (models per role, reasoning effort, output format), the mission branch name, the worktree policy, the gate command, the hooks that must be active, the usage budget per milestone, what the human will and will not answer mid-mission
## 3. Milestones         Walking skeleton first (core crate, canonical event, replay of one capture, empty board). Then one milestone per PRD section in the order the PRD gives: for each, the tests it turns green, its acceptance commands, its gate, its exit criterion, what it must not touch. M0 freezes the stream mapping before later milestones are planned in detail. A milestone is integrated into the mission branch, never shipped
## 4. Versions           table: component, version, source, verified date (crates, Tauri, toolchain, images, actions)
## 5. Architecture       crates and their interfaces, the canonical event and lane mapping (versioned), the store schema, the ingest adapters behind traits, config keys with defaults, metric definitions by reference to the PRD, runtime limits, security (keychain, read-only raw lines, the one outward write)
## 6. Tests              per suite: what it covers, where it runs (local / mission branch CI / final PR), the command; fixtures are captured streams, named and dated
## 7. CI                 jobs on the mission branch (light, under three minutes warm) and on the final pull request (full, attested); what each gates; required checks
## 8. Deliverables       README contents, docs/decisions/ (one file per decision, generated index), BACKLOG.md, build-log/ (hook logs), ai/session/ (session log with transcript links), reports/ (usage and waste per milestone, pasted from the tool)
## 9. Changelog and human feedback   | Id | Date | Feedback | Applies | Status |   (written by the orchestrator on the mission branch, never in a worktree)
## 10. Evolution log     one row per integrated milestone: what landed, tests green, usage spent, deviations, unknown events left   (written at integration, from the workers' and validators' reports)
```

Each decision in §5 carries a one-line reason. Numbers come from the PRD unless the spec says why they changed. The spec is written by a Droid in an interactive spec session (PRD, review notes, RULES.md and this template as inputs) and approved by the human before the mission starts; the mission plans from the approved spec, it does not write it.
