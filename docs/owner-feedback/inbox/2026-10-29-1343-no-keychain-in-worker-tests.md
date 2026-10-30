# Rule, apply now: no worker test reads the keychain; live forge checks are owner-run

Source: pre-roci under the owner's delegation; triggered 13:41 by a macOS keychain prompt from the worker of m4-forge-core-and-pr-detection: a cargo test binary `forge_live-1607b840b0124955` asked for `droid-observatory/github-token`. The owner saw the prompt and asked what it was. He decides the dialog; this rule stops the recurrence.

1. A test that needs a secret from the keychain is an owner-run row (the `attended` class of M3 item 18), never part of a worker's gate or of `make check`. It is `#[ignore]` unless the owner sets `FORGE_LIVE=1` and runs it himself in his terminal; the token reaches it only through the owner's invocation, never through a worker.
2. A worker whose test would prompt the owner's GUI marks the row as owner-run in its hand-off and moves on; it never waits on a dialog. If a prompt already appeared, the worker's live check counts as not run, whatever the dialog's answer.
3. Each rebuilt test binary has a new code hash, so "Always Allow" binds the secret to a throwaway executable; the owner will deny or allow once, not always. Record this as a tool fact (macOS keychain ACL is per binary hash).
4. The read-only PAT stays in the keychain (owner's Q2); nothing here changes where the secret lives.

Apply from this feature: the orchestrator passes the rule to the running worker at its next boundary (rule 37 style), and to every later feature that touches the forge (m4-forge-*, m5-integrate-m4). Cycle rule; KB candidate for R-025 and the security section of the edition.
