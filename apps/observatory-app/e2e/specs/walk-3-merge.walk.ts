import { expect } from '@wdio/globals';
import { browser } from '@wdio/globals';
import { query } from '../lib/attach.js';
import { startForgeProxy, type ForgeStub } from '../lib/forge.js';
import { byId } from '../lib/hosted.js';
import { gh, openAttached, keepLog, png, savedPr, sh, WALK } from '../lib/walk.js';

describe('live walk 3: Merge pressed, 403, the owner merges', () => {
  let forge: ForgeStub;
  let pr = 0;
  before(async () => {
    forge = await startForgeProxy();
    pr = savedPr();
    await openAttached();
  });
  after(async () => {
    await forge.close();
  });

  it('presses Merge twice: confirmation, 403 text, one request and one row per press', async () => {
    await byId('merge').waitForDisplayed({ timeout: 120_000, timeoutMsg: 'no Merge button' });
    await byId('merge').click();
    await byId('merge-confirmation').waitForDisplayed({ timeout: 30_000 });
    await expect(byId('merge-confirmation-text')).toHaveText(`Merge PR #${pr} into ${WALK.base}?`);
    await png('live-confirm-dialog', byId('merge-confirmation'));
    await byId('merge-confirm').click();
    await byId('merge-result').waitForDisplayed({ timeout: 30_000 });
    await expect(byId('merge-result')).toHaveText(expect.stringContaining(`Merge of PR #${pr} into ${WALK.base} refused: token lacks pull_requests:write.`));
    expect(forge.requests.filter((line) => line.startsWith('PUT '))).toHaveLength(1);
    await png('live-403-message', byId('merge-result'));
    await byId('merge').click();
    await byId('merge-confirm').click();
    await browser.waitUntil(() => forge.requests.filter((line) => line.startsWith('PUT ')).length === 2, { timeout: 30_000 });
    await browser.pause(1_500);
    expect(forge.requests.filter((line) => line.startsWith('PUT '))).toHaveLength(2);
    keepLog(['merge_requested', 'merge_result', 'github.request', 'human_action']);
    expect(query("SELECT kind, state, target FROM human_actions WHERE kind = 'merge'")).toHaveLength(2);
    expect(query("SELECT COUNT(*) FROM events WHERE kind = 'approval.given' AND overlay = 'human' AND lane = 'gate'")).toEqual(['2']);
    sh('sqlite3', '-readonly', `${process.env.E2E_RUN_DIR}/observatory.sqlite`, "SELECT id, kind, state, target, at FROM human_actions; SELECT id, kind, overlay, lane, role, source FROM events WHERE kind='approval.given'");
    await png('live-two-presses', byId('gate-group'));
  });

  it('the owner merges with gh and the badge reads handed to CD within one poll interval', async () => {
    const started = Date.now();
    gh('pr', 'merge', String(pr), '--repo', WALK.repo, '--squash', '--delete-branch');
    const mergedAt = new Date().toISOString();
    await browser.waitUntil(async () => (await byId('gate-badge').getText()) === 'handed to CD', {
      timeout: 90_000,
      timeoutMsg: 'the badge never read handed to CD',
    });
    const seen = (Date.now() - started) / 1000;
    sh('echo', `merge at ${mergedAt}; badge handed to CD ${seen}s after the merge command started`);
    const mergeSha = gh('pr', 'view', String(pr), '--repo', WALK.repo, '--json', 'mergeCommit,mergedBy,state', '--jq', '.mergeCommit.oid + " " + .mergedBy.login + " " + .state');
    const short = mergeSha.slice(0, 7);
    await expect(byId('gate-handed-to-cd')).toHaveText(expect.stringContaining(`handed to CD: ${short} · PR #${pr}`));
    await expect(byId('gate-handed-to-cd')).toHaveText(expect.stringContaining('merged by human'));
    await expect(byId('merge')).not.toBeExisting();
    expect(seen).toBeLessThan(45);
    keepLog(['gate_changed', 'github_poll']);
    sh('sqlite3', '-readonly', `${process.env.E2E_RUN_DIR}/observatory.sqlite`, "SELECT number, state, head_sha, merged_by, url FROM prs; SELECT id, kind, overlay, lane, role, source, sha, url FROM events WHERE kind LIKE 'handed%'; SELECT * FROM attestations");
    await png('live-handed-to-cd', byId('gate-group'));
    sh('git', 'ls-remote', '--heads', 'origin', WALK.head);
  });
});
