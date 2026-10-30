# Row 62 (a) and (b) answered: the derivation behind the hand ratios, and the definition the usage report should pin

Source: owner's delegate (pre-roci), 01:45, 2026-10-30. Record, not a slice.

## (a) Why three readings of M4's ratios differ
The decomposition is cost per assertion = w x (1 + coordination + validation + tooling), and every reading of it today used a different base for w:
- Hand figures at 23:40 (1.06 / 2.86 / 1.30 / 0.85): the day's log only (09:21 to 21:09 local), w = worker cost on contract features taken as 54 % of all worker cost (the D-045 turn share 823 of 1525), orchestrator own session 100, validators 45, tooling workers 30, on 33 assertions.
- Regenerated pages at 00:25 (0.54 / 5.99 / 2.82 / 1.81): build window 10:25 to 23:30 local, w = worker cost on features whose `fulfills` names at least one contract assertion (strict rule), orchestrator with its subagents 106.65, validators with their seats, tooling = worker cost on features claiming none, integration as its own term 0.40.
- The usage report's script (0.87 / 4.64 / 1.80 / 0.86, worker 27.85): its own window and attribution, which the report should state next to the table.
None is wrong; they answer three questions. The report has to pick one and name it.

## The definition to pin (owner's delegate, for the report and for Roci 10.3b)
Window: the milestone's build window, first own worker dispatch to integration landed. w = worker cost on features that claim at least one contract assertion, divided by the contract assertions landed. Coordination = orchestrator cost with its subagents / that worker cost. Validation = scrutiny and user-testing validators with their seats / that worker cost. Tooling = worker cost on features that claim no contract assertion / that worker cost. Integration = the integrator's cost / that worker cost, shown as its own term. The report prints the four bases in a line under the table so a reader can rebuild the figure. Weighted variants (plan turn budget; kinds 1.0 / 1.25 / 1.5) are shown next to the raw.

## (b) Kinds 12 / 14 / 6 against 12 / 15 / 6
The product's count stands: the fifteenth behavioural assertion in the hand file was one of the two blocked ones; blocked is not landed. The hand file (`resume/roci/kinds-after-the-fact.json`) will be corrected in the morning.

## (c)
The report size and the red check-scripts at the tip are the product's housekeeping; no owner input.
