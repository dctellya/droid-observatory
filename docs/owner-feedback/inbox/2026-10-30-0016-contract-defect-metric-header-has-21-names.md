# Contract defect for the next contract pass: VAL-METRIC-001 says "20-column" but lists 21 names

Source: owner's delegate (pre-roci), 00:16, 2026-10-30, found while drafting the B-008 reference CSV independently.

The header line in VAL-METRIC-001 (and the one VAL-METRIC-002 and -011 refer to) has 21 names: scope, id, name, cycle_s, first_pass_active_s, rework_s, the eight waste_*_s, rework_windows, the four tokens_*, cost_usd, price_table (3 + 3 + 8 + 1 + 4 + 2 = 21). The literal header is the contract; the word "20-column" is the error (architecture §9's 22-name header adds throttled_minutes, which is a different file). Fix the label in VAL-METRIC-001 and -002 at the next contract pass, before m5-metrics-report-and-oracle runs; nothing else changes. Record, not a slice.

For the morning: a draft of the reference CSV (19 rows: 12 features, 7 milestones; header byte-identical) with its method and 15 review points is ready for the owner at `resume/roci/oracle-draft/`; it enters the tree only on the owner's word.
