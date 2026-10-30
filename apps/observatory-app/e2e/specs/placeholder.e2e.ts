import { $, browser, expect } from '@wdio/globals';
import { E2E_BRANCH as BRANCH, WAIT_MS } from '../lib/env.js';
import { ATTACHED_ID, appLog } from '../lib/attach.js';
import { openAttached } from '../lib/walk.js';

describe('the pre-gate placeholder of an attached mission', () => {
  before(async () => {
    await openAttached();
  });

  it('names the configured mission branch before any poll', async () => {
    const branch = $('[aria-label="gate-placeholder"] [aria-label="gate-branch"]');
    await branch.waitForDisplayed({ timeout: WAIT_MS, timeoutMsg: 'the board shows no Gate placeholder' });
    await expect(branch).toHaveText(`mission branch: ${BRANCH}`);
  });

  it('names the same branch in the pending gate_changed line of the app log', async () => {
    const answer = await browser.executeAsync((mission_id: string, done: (value: string) => void) => {
      (window as any).__TAURI__.core
        .invoke('e2e_gate_poll', { scenario_name: 'pending', mission_id })
        .then((mission: string) => done(`ok ${mission}`), (error: unknown) => done(`error ${error}`));
    }, ATTACHED_ID);
    expect(answer).toMatch(/^ok /);
    const line = await browser.waitUntil(
      () =>
        appLog().find(
          (entry) =>
            entry.includes('gate_changed') && entry.includes(`mission_id=${ATTACHED_ID}`) && entry.includes('badge="pending"'),
        ) ?? false,
      { timeout: WAIT_MS, timeoutMsg: 'the app log has no pending gate_changed line' },
    );
    expect(line).toContain(`branch=${BRANCH}`);
    await expect($('[aria-label="gate-placeholder"] [aria-label="gate-branch"]')).toHaveText(`mission branch: ${BRANCH}`);
  });
});
