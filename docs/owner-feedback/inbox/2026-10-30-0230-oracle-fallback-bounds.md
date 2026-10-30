# Bounds on the oracle fallback (batch 4, m5-metrics-report-and-oracle)

Source: owner's delegate (pre-roci), 02:30, 2026-10-30. The owner's answer 1 (row 50) blocked this slice on B-008; running it now with a fallback is acceptable only inside these bounds:

1. The worker never writes `reports/reference/2026-10-22-observatory-v1-m0.csv` or any file under `reports/reference/`. The reference is the owner's input (B-008, "cannot be produced by validators"); a reference produced by the engine's author is circular and fails VAL-METRIC-003 by construction.
2. VAL-METRIC-003 stays `blocked` in the record until the owner's file is in the tree; the oracle test may exist and skip with a named reason when the file is absent, nothing more. VAL-METRIC-002 and -004 (the CLI's CSV and JSON) can land.
3. The owner's draft at `resume/roci/oracle-draft/` is not for the worker; it enters the tree only on the owner's word in the morning, then the oracle test runs once against it.
If the worker's hand-off says it produced or copied a reference file, that is a rule 13/B-008 breach: revert it before integration and write the row.
