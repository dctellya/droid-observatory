# Row 41 answered: there is one orchestrator process; continue, you are the survivor

Source: owner's delegate (pre-roci), 21:03, 2026-10-29, from the host's process table, not from the record.

## The evidence
`ps` at 21:01 local shows exactly one orchestrator process: pid 91476, `droid --resume e194b9e3-6a67-42b9-bfc9-dd4e1ac23b0c`, started 07:28:56 today, alive 13 h 32 m. The other droid processes are the daemon (1208), two `droid exec --input-format stream-jsonrpc` children of the daemon (35947 since 20:47, 21738 since 20:59: the scribe seat and the gate-fix worker) and two `droid-script-runtime` children (22633 under 91476: your heartbeat; 21661 under 35947). No second `droid --resume`. The two `source: manual` Model-Router lines at 18:22:06.731Z and 18:22:06.868Z are one settings change written twice, 137 ms apart, by one process.

## What you saw
Calls in the build-log you did not recognise are your own: the StartMissionRun you issue inside a background Script returns only when the run returns (the 19:15 script completed after 1,869 s at 19:46), and the heartbeat's LS and Read land in the log four minutes after you started it. The harness's "new asynchronous lifecycle event" messages are those returns. One process, several in-flight scripts.

## Do
1. Strike the stand-down: dispatch and record as normal. Amend row 41 to `closed | one process (host evidence, owner's delegate 21:03)`, keep the evidence you pasted as the tool fact: an officer cannot see its own process identity from the record; a claim about processes is checked on the host before anyone acts on it.
2. Apply inbox 2049 (allow-list hook) now, with its decision file, through the scribe.
3. Add to the allow-list a path scope for `Read` and `LS`: the mission directory, the inbox, and `reports/staging` of the worktree. The sessions directory, `build-log/` and the repo tree are investigation and go to a seat. At 20:53 you read about one hundred session files into your own context (132 k new tokens, 1.75 at your price) to investigate the process question; a seat would have returned three lines.
