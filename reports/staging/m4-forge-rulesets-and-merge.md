# Staging: m4-forge-rulesets-and-merge

Feature `m4-forge-rulesets-and-merge` (VAL-GATE-015, -016, -027, -028; slice of `m4-github-forge-core`), worktree
`observatory-wt/m4`, branch `wt/m4`. VAL-GATE-006 (the eight named tests) is claimed by `m5-integrate-m4`.

## SPEC §9

- Row (F-NEW): `Forge` gains `policy(target)`, `reviews(pr)`, `attestations(pr)`, `token_owner()` and `merge(pr, method)`
  (architecture §4.6 lists them). `policy` reads `GET /rules/branches/{target}`, names the ruleset from `GET /rulesets`
  (`ruleset {id}` when that list is unreadable) and falls back to `GET /branches/{target}/protection` (404 is no protection):
  `RuleAttribution::{Ruleset{id,name}, BranchProtection, ConfigOverride(text), NoRule}`. `requires_human` is "a
  `pull_request` rule applies"; the required checks are the contexts of the `required_status_checks` rules.
  `[github].merge_policy` is applied by `MergePolicy::with_override`: `auto` keeps the forge's answer unless auto-merge is on
  for the PR, `human` forces `requires_human` and names `ConfigOverride("human")`, `policy` clears it and names
  `ConfigOverride("policy")`; the required checks stay.
- Row (F-NEW): `Poll` carries `reviews`, `attestations` and `gate: Option<GateFacts>` (policy in force, one `RequiredCheck` per
  required name with its conclusion, `RequiredMissing` when nothing reports it, review count, attestation ids, `pr_open`).
  `GateGroup` gains `decided_by`, `requires_human`, `merge_offered`, `required_checks`, `reviews`, `attestation_ids` (all
  `#[serde(default)]`), filled by `GateGroup::with_facts`. `merge_offered` = PR open, a person decides, badge `ci_green` and every
  required check `passed` (`merge_button_shows_only_when_policy_is_human_and_checks_are_green`). The shell's `gate_changed` log
  line and window event now carry the header with the facts and fire when any of it changes (a policy flip, not only the badge).
  Policy, reviews and attestations that cannot be read degrade (warn, no facts); a failure there never fails the poll, except
  the attestations of a merged PR (below).
- Row (F-NEW): `Forge::merge` PUTs `{"merge_method": ...}` to `/pulls/{n}/merge`, logged as `github.request{method="PUT", url, status}`
  like every request (no body, no header). A 403 that is not the rate limit is `ForgeError::Forbidden { pr, target,
  missing_permission: "pull_requests:write" }`; one PUT, no retry. `HttpTransport::put` has a default that refuses, so a read-only
  transport needs nothing.
- Row (F-NEW), `handed_to_cd` (F-028 amendment): when the poll sees `PrState::Merged` and every required check is `passed`
  (all observed checks when the policy is unreadable) it emits `github:pr:<n>:handed_to_cd` once: `merge_sha`, `pr_number`,
  `attestation_ids`, `at` = `merged_at`, pointer = the PR API URL, `role = Human` when `merged_by` equals the token owner
  (`GET /user`, read once) else `External`. `merged_by` comes from `GET /pulls/{n}` (the list omits it); that call, the
  token-owner call and the attestations of a merged PR fail the poll instead of degrading, because the event is announced once
  and a wrong role or empty id list would stay. Branch deletion and anything after the merge are not observed.
- Row (F-NEW), attestations: `Forge::attestations` lists the artifacts of the newest passed run of each workflow on the head
  (`GET /actions/runs/{id}/artifacts`, 404 is none) and asks `GET /attestations/{digest}` for each artifact digest (404 is none);
  the id is the forge's `id` when it gives one, else `<digest>#<position>`. `verified` is false: nothing verifies a bundle here.
  The demo repository has no artifacts, so its attestations are an empty list (count 0), as VAL-GATE-014 wants shown.
- Row (F-NEW), store: migration `0004_attestation_ids.sql` adds `attestations.attestation_id` (unique per PR). `record_poll` writes
  `prs.merged_by`, one `reviews` row per GitHub review id and one `attestations` row per (PR, id), and on a `handed_to_cd` event
  sets `missions.state = 'complete'`, `completed_at` = the event's `at` (only while `completed_at` is NULL). Inserting a
  `handed_to_cd` now writes `events.sha` = the merge sha; reading it joins the attestation ids of its PR, so the round trip is
  exact (the lossy case noted by `m1-view-models-core` is closed). `Store::events` and `Store::event` both fill the ids.
- Evidence (live, 2026-10-29, read-only, `GITHUB_TOKEN="$(gh auth token)"`, ignored test
  `the_live_gate_names_the_rule_and_records_a_merge` against PR #24 `demo/smoke-1 -> demo/base`, closed): requests
  `pulls?head=...`, `actions/runs?head_sha=`, `commits/<sha>/status`, `actions/runs/40853894595/artifacts`, `pulls/24/reviews`,
  `rules/branches/demo/base`, `rulesets`, all `status=200`, no check-runs; policy `auto`: `decided_by Some(Ruleset { id: 26112925,
  name: "demo-gate" }) requires_human true`; policy `policy`: `decided_by Some(ConfigOverride("policy")) requires_human false
  merge_offered false`; the PR is closed so `merge_offered` is false in both; `grep -c 'Authorization\|gho_\|github_pat'` of both
  outputs: 0.

## Decisions

None.

## BACKLOG

- `check.required_missing`, `ruleset.blocked` and `pr.review_requested` are not emitted: `required_checks` carries
  `RequiredMissing` in the header facts, but a required check with no run yet is normal right after the PR opens and an event
  for it would flip the badge to `blocked` and back. A slice that wants the `blocked` badge needs a settle rule (no result for
  N polls), and a config key for N.
- `tests/forge.rs` and `tests/forge_merge.rs` each carry their own recorded-route `Replay` transport; a shared module under
  `tests/common/` would remove the copy.
- Reviews are read from the first page only (30); a PR with more reviews under-counts. Unlikely on the demo pair.
- The attestation lookup needs the runs' artifacts to carry a `digest`; artifacts older than the API's retention have none and
  yield no attestation. Whether PR-level attestations should come from another endpoint is the live walk's question.

## Deferred clauses

- The live walk of VAL-GATE-015/-016/-027/-028 on a real merge (a second `demo/*` PR merged with `gh pr merge`, the store rows
  of `prs.merged_by`, `handed_to_cd` and `missions.completed_at` pasted) is not run here: a merge by the token owner reads `human`,
  and the `external` row needs a merge by another login, which this worker does not have; the demo PR itself is the validator's
  (`m4-forge-live-walk-2`). The ignored test above is the harness for it (`OBSERVATORY_LIVE_POLICY`, `OBSERVATORY_LIVE_STORE`).
  The unit tests replay the recorded PR 24 as merged by a colleague (`external`, no `human_actions` merge row) and by the owner
  (`human`), through the real source, store and view.
- The Merge button itself, the confirmation dialog and `control::merge` are `m4-merge-ui-and-403`; this slice gives it
  `GateGroup.merge_offered`, `Forge::merge` and the 403 text.
- Coverage of `observatory-core` (80 %) runs once at the integration.

## Library

- `GET /pulls/{n}` is the only endpoint that names `merged_by`; the list leaves it out.
- `GET /rules/branches/{branch}` returns the rules with `ruleset_id` but not the ruleset name; the name is in `GET /rulesets`.
  Recorded for the demo pair in `fixtures/github/demo-smoke-1/{rules,rulesets}.json`.
- A poll now makes eight requests on an open PR with a passed run (pulls, runs, status, runs again for the attestations, the
  run's artifacts, reviews, rules, rulesets): about 16 a minute at the 30 s interval.
- `scripts/contract-checks/g2-tests.tsv`: rows for `merge_button_shows_only_when_policy_is_human_and_checks_are_green`,
  `rule_attribution_names_the_ruleset` and `policy_merged_detected_without_human_action` flipped to `yes`; all eight named
  gate tests are now `yes`.
