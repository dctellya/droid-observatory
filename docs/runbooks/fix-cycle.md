# Fix cycle

**Symptom:** the mission has ended (orchestrator reported `mission.completed`) but CI on the
mission PR is red, or a check turned red after a rerun.

The fix is a new cycle on the **same branch and the same PR** (PRD §2, D-001). Nothing is
reopened on `main`.

## Steps

1. Label the PR `fix-cycle`. Read the failing job output (see [red-gate.md](red-gate.md)).
2. Write the fix mission file: the affected feature's spec section, the failing check output
   pasted verbatim, the mission branch name, and the original session ids
   (`~/.factory/sessions/<cwd-slug>/`). The observatory pre-fills this from M4 onward; until
   then copy it by hand.
3. Start the fix in the repository checkout on the mission branch:

   ```sh
   git switch mission/<name>
   droid exec --mission --auto high \
     --model claude-fable-5-1 --worker-model claude-sonnet-5-5 --validator-model claude-opus-5-5 \
     -f fix-<name>.md
   ```

4. The fix commits land on the mission branch as new commits; the PR updates; the gate
   reruns. Record the cycle in the session log and in `reports/` (time and tokens pasted
   from the tool).
5. Merge only when `gate` is green, by the human.

## Stop and involve the human when

- The failure is in a required check that does not belong to this repository (GitHub
  outage, action removed upstream).
- The fix would change the spec: propose it with reason and cost first (rule 9).
