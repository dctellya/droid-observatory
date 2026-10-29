import { expect } from '@wdio/globals';
import { browser } from '@wdio/globals';
import { startForgeProxy, type ForgeStub } from '../lib/forge.js';
import { byId } from '../lib/hosted.js';
import { gh, openAttached, keepLog, png, savedPr, WALK } from '../lib/walk.js';

describe('live walk 2: the same green pull request under merge_policy = "policy"', () => {
  let forge: ForgeStub;
  before(async () => {
    forge = await startForgeProxy();
    await openAttached();
  });
  after(async () => {
    await forge.close();
  });

  it('shows CI green and no Merge button', async () => {
    const pr = savedPr();
    gh('pr', 'view', String(pr), '--repo', WALK.repo, '--json', 'number,state,labels');
    await browser.waitUntil(async () => (await byId('gate-badge').isExisting()) && (await byId('gate-badge').getText()) === 'CI green', {
      timeout: 120_000,
      timeoutMsg: 'the badge never read CI green',
    });
    await expect(byId('merge')).not.toBeExisting();
    const lines = keepLog(['gate_changed']);
    expect(lines.some((line) => line.includes('merge_offered=false'))).toBe(true);
    await png('live-policy-no-merge', byId('gate-group'));
  });
});
