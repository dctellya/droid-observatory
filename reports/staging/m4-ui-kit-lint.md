# Staging: m4-ui-kit-lint

Feature `m4-ui-kit-lint` (owner item 14b, second half), worktree `observatory-wt/m4`, branch `wt/m4`.

## SPEC §9

- Row (F-NEW), rule 28 amendment (owner item 14b, F-151): `make check-kit` is the UI-kit lint (`scripts/check-kit.py`, Python
  stdlib, 0.07 s). It reads `apps/observatory-app/ui` (`src/*.rs`, `style.css`, `index.html`, `theme.css`) against
  `ai/design/ui-kit-v2/` and prints `check-kit ok|FAIL VAL-UI-001`. Rules: (1) `ui/theme.css` equals the kit's, no literal
  colour, no `font-family`, no inline spacing or type style outside it; (2) text that is identifier-, timestamp- or
  number-shaped (the rt3 `scan.js` shapes, applied to the `view!` templates: literal text by the scan's regex, Rust
  expressions by the names they read, `<time>` always) sits in an element with a mono-face class (any class that sets
  `font-family: var(--mono)` or `tabular-nums` in the kit or in `style.css`), a number-shaped `td` carries `n`, and every
  `<table>` sits in `.lib`; (3, 6) a `padding`, `margin` or `gap` on a selector the kit also styles, `padding` on any
  `th`/`td`, any `max-width` other than 1180 px, any `inset` and a font size off the 11/12/14/15/22 scale is a departure that
  must be a row of `apps/observatory-app/kit-departures.md` (selector, property, kit value, actual, reason, SPEC row), and a
  row whose departure is gone, or whose values are stale, is red. Wired: `check-all` (cached by the gate, code paths), a `kit`
  job in `mission-branch.yml` (in `mission-checks.needs`, required by `check-ci-jobs.sh`), and the self-test
  `scripts/tests/kit_lint_test.sh` in `check-scripts`. Paths: the app's UI lives in `apps/observatory-app/ui/`, not
  `apps/observatory-app/src` and `style.css` as the feature text says.
- Row (F-NEW), departure found by the lint and not in §9 before: `ui/style.css:48`, `.card { padding: 6px 10px; gap: 4px }`
  against the kit's `.card { padding: 12px 14px; gap: 8px }` (kit rule 6, board and stall-banner cards). Two rows of
  `kit-departures.md` with SPEC row `F-NEW`. Owner decision needed: approve as a departure, or revert in `m4-gate-group-ui`
  (then the two rows go).
- Row (F-NEW), the kit table after `m4-gate-group-ui` lists F-105 only (`.observatory`, `.library`, `.details`: 3 rows,
  one approved departure) once that feature reverts F-103 and F-104 (rows `.library th, .library td` and the
  `.panel th ... .details td` group go with the override) and the `.card` question above is closed.

## Decisions

None.

## BACKLOG

- New: `apps/observatory-app/kit-pending.txt` lists 15 known rule 2 violations the lint tolerates until fixed (each one an
  identifier-, timestamp- or number-shaped text in a sans element): the five heading counts of the UT round 3 note (agents
  `agent-count`, details `details-milestone` and `details-row-count`, panel `overlay-human-count`, tray `tray-count`; owner
  `m4-gate-group-ui`) and ten the note did not name: agents `agent-stream-rows` and `agent-stream-markers`, attach and launch
  `<code>` session and request ids, launch `hosted-pid` and `live-output-total`, controls `feature.0` span (feature marks; mono
  only through its parent `ul`), raw `raw-line-event` and `raw-line-kind`. Fix: add `class="mono"` on the element, delete its
  line from the file. The lint is red when a fixed line stays listed, so the list shrinks and cannot grow unseen.
- New: the static lint cannot see a parent in another `view!` (it asks for `class="mono"` on the element itself); a runtime
  probe stays the validator's tool for faces that come from a distant ancestor (`evidence/m3/rt3/probe.js`).

## Deferred clauses

None. VAL-UI-001 clauses (1) tokens, (2) two faces, (6) departures are read from the line `check-kit ok VAL-UI-001` in the gate
record; the validator still grades (3) the five components, (4) the donut and (5) no new navigation, which are behaviour and
structure, not style rules.

## Library

- `make check-kit` is a sub-second inner-loop check for any UI feature; `KIT_ROOT=<scratch tree>` points it at a copy
  (`apps/observatory-app/ui`, `ai/design/ui-kit-v2`, `kit-departures.md`, `kit-pending.txt`).
- Adding a departure: a row in `apps/observatory-app/kit-departures.md` in the same commit as the CSS; reverting one: delete the
  row in the commit that removes the override.
- A mono violation the lint names is fixed by `class="mono"` on the element (numbers in a `td` also `n`); a line left in
  `kit-pending.txt` after the fix is red.
