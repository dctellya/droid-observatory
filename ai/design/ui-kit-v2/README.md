# Roci UI kit v2 (25-oct-2026)

For the droid-observatory v2 screens (M3 controls UI, M4 gate UI, and the existing board). Not the full Roci product design; the part of it that makes any observatory screen read well: tokens, type, spacing, five components, one donut. Apply it to the structure the app already has.

## Files

- `theme.css`: tokens for both themes (dark first, follows the system, `data-theme` override), base type, and the components below. Lane colours `--l-plan … --l-gate`, cause colours `--w-*`, state colours `--ok --warn --bad --info`, Roci orange `--accent`.

- `donut.js`: the Option B donut as a function over an SVG (`donutB`), plus `ring`, `arc`, `el`, `txt`. Centre shows cost (large) or value share (small, in tables).

- Reference: `../roci-app-mock.html` (v49) for how the pieces sit together; `../roci-views-mock.html` for the trace, the post-mortem by feature and the Pareto.

## Eight rules

1. **Tokens only.** No literal colours in components; every colour is a token so both themes hold. Lanes and causes keep their colours everywhere (donut, trace, Pareto swatch, legend).
2. **Two faces.** IBM Plex Sans for UI text (400/500/600). JetBrains Mono for everything that is an identifier or a number: ids, event kinds, tokens, USD-eq, minutes, timestamps, chips, table numerics (`tabular-nums`, right-aligned).
3. **One scale.** Sizes 11, 12, 14 (body), 15 (h3), 22 (page title). Spacing 4, 8, 12, 18, 28. Page max width 1180 px; rail 220 px.
4. **Every page starts the same way.** `.eyebrow` breadcrumb in mono orange (`Products › observatory-v1 › mission e194b9e3`), an `h2` title, a mono muted subtitle on the right with the facts (system, template, elapsed, window). Then KPI tiles, then the table or board.
5. **KPI tiles say one number each**: mono value with its unit in small muted text, a plain-language caption below. Red value only for a bad number (tokens burnt in waste, cards needing a decision).
6. **Tables are the lists.** `.lib` box, uppercase 11 px headers, 10 px cells, the name column bold with a mono small line under it, numbers right-aligned in mono, a state chip, and a 90 px mini donut as the last column where there is lane data. Rows are clickable (hover tint); rows without a record are dimmed, never hidden.
7. **Chips and switches carry state and choice.** Chips: `run` green, `pause` amber, `bad` red, `done` muted. Pill switches for the units of a cycle, the tabs of a page (Watcher · Post-mortem · Continuous improvement), and the theme.
8. **Surfaces are quiet.** Background `--bg`, panels `--surface` with a 1 px `--border` and 10 px radius, no shadows except the brand mark. Dark mode is not an inversion: the tokens are tuned per theme; keep 4.5:1 for text everywhere.

## What not to do

- No new navigation; the app's structure stays (products › runs › missions when it exists, otherwise the current screens).

- No brand work: the mark and the badge are provisional (`../refs/`); use the wordmark text "Roci" in the rail and nothing else.

- No per-screen colour schemes, no gradients, no icon packs; lanes and causes are the only colour.

## The donut, in one paragraph

Inner ring 16 px: the lane's value share in the lane colour, the rest in `--ring-bg`. Outer ring 8 px, starting where value ends: one slice per cause in its `--w-*` colour; throttled is not a slice but a dashed arc outside the ring (a gap: nobody was working). Centre: cost in USD-eq (or tokens when toggled) at 20 px mono; the caption under the donut reads "71% value · 31 min". Small variant (tables): 14 px inner ring, value share in the centre, no caption. A lane not started draws a dashed empty circle and says so.
