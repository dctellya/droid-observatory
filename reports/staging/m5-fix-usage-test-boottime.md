# m5-fix-usage-test-boottime

## SPEC §9

- Row (F-NEW): **usage report tests pinned to a fixed boot time** (fix slice after `final-integrate-m5` cb505f95 hit a red gate; rule 29, rule 24, no scope, cap or model change). `scripts/tests/usage_test.py` ran `scripts/usage-report.py` without `--boottime`, so `### Host health` read the host's own `sysctl kern.boottime`; the host rebooted 2026-10-30T07:12:35Z, after the fixture window end (2026-10-25), and `scripts/usage_idle.py` printed "booted after the window end: ... its uptime is not recorded", which failed `test_the_gates_table_counts_runs_and_cache_hits_per_gate_and_in_total` (`assertNotIn("not recorded")`). All three report runners of the file (`run`, the default-window test, `UsageReportPriceFiles.run_in`) now pass `--boottime` with one constant, `BOOTTIME` (2026-10-20T00:00:00Z, before the window), so the suite no longer depends on the host's uptime. `scripts/usage_idle.py` and `scripts/usage-report.py` are untouched: `--boottime` was already plumbed. Alternative: drop the `not recorded` assertion (rejected: it guards the gates table against a stray host-health line, and the next reboot would still change `Host health`).

## Decisions

None.

## BACKLOG

- Closed (do not file): **usage_test boot-time dependency**. `usage_test.py` no longer reads the host's boot time; `test_the_report_runner_pins_the_boot_time_so_the_host_uptime_never_reaches_a_test` fails if a runner stops passing `--boottime`.
- Not filed: `usage_idle_test.py` already pinned `--boottime` in every run; the other report test files (`usage_ratios_test.py`, `usage_rewrites_test.py`, `usage_testing_test.py`) pass with the host as it is today (exit 0).

## Deferred clauses

- None.

### Claims (rule 30)

None: `fulfills` is empty (infra fix, no contract assertion). Evidence for each `expectedBehavior` line:

| Clause | Evidence |
|---|---|
| `python3 scripts/usage_test.py` exits 0 whatever the host boot time | `python3 scripts/tests/usage_test.py` exit 0, `Ran 83 tests`, `OK`; the report's Host health row `Boot time (UTC)` reads `2026-10-20T00:00:00.000Z`, the pinned value |
| No behaviour change outside the harness | `git diff --stat` of the feature touches `scripts/tests/usage_test.py` and this file only |
| Staging file passes the check with `###` headings | `python3 scripts/integrate-staging.py --check` |

Red/green: red commit `e813023` (`python3 scripts/tests/usage_test.py` exit 1, FAILED failures=2 of 83: the gates table test and the new pin test) to the green commit of this file.

## Library

- A test that builds a usage report must pass `--boottime` (the `BOOTTIME` constant of `usage_test.py`), or `### Host health` carries the machine's uptime and the output changes at the next reboot.
- `make -n check` is not a dry run of the gate: its `scripts/gate.sh` line runs (make expands the `$(MAKE)` recursion) and overwrites the tree's gate record. Use `make -n check-all` or read the Makefile instead.
