# UI kit departures

Every place where `ui/style.css` departs from the UI kit v2 (`ai/design/ui-kit-v2/`, `theme.css` and its
README "Eight rules"). `make check-kit` (`scripts/check-kit.py`) reads this table and fails when `style.css`
departs without a row, when a row's kit value or actual value is not what the files say, when a row names a
departure that is gone, and when a row has no reason or no SPEC row. A departure is, in `style.css`:

- a `padding`, `margin` or `gap` on a selector the kit also styles (same selector text), or `padding` on any
  `th` or `td` (the kit's `.lib td` is `10px`), with a different value;
- any `max-width` other than the kit's page width (`1180px`) and any `inset`;
- a `font-size` off the kit scale (11, 12, 14, 15, 22 px).

`Selector` is the selector list of the rule as `style.css` writes it. `SPEC row` is the §9 row that approved or
proposed the departure (`F-NEW` waits for a row number). Delete a row in the commit that removes its
departure; add a row in the commit that adds one, with the reason.

| Selector | Property | Kit value | Actual | Reason | SPEC row |
|---|---|---|---|---|---|
| `.observatory` | max-width | 1180px | 1480px | Kit rule 3 (page width): the board's seven lanes need the room; at 1180 px each lane is about 150 px wide. Screen: the board. Approved at the M3 boundary. | F-105 |
| `.library` | max-width | 1180px | 960px | Kit rule 3: the Library box is narrower than the page, a list of captures. Part of the same approved departure. | F-105 |
| `.details` | inset | none | 5vh 3vw | Kit rule 3: Details is a fixed overlay inset 3 vw, narrower than 1180 px; the kit has no overlay. Part of the same approved departure. | F-105 |
| `.library th, .library td` | padding | 10px | 6px 8px | Kit rule 6 (table density): the Library lists every opened capture, 10 px rows halve how many show without scrolling. Reverted by `m4-gate-group-ui` (owner answer Q4, 2026-10-25); a denser table only as one kit variant class. | F-103 |
| `.panel th, .panel td, .agents th, .agents td, .details th, .details td` | padding | 10px | 3px 8px | Kit rule 6: row density on the board page, where the Panel, the Agents table and the Details timeline share one viewport with the lanes. Reverted by `m4-gate-group-ui` (owner answer Q4); a denser table only as one kit variant class. | F-104 |
| `.card` | padding | 12px 14px | 6px 10px | Kit card padding: the board's cards and the stall banner's cards are denser so a lane shows more of them. Found by `make check-kit` (`m4-ui-kit-lint`); not approved by the owner yet. | F-NEW |
| `.card` | gap | 8px | 4px | Same override as the card padding above. | F-NEW |
