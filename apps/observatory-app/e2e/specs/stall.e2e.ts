import { browser, expect } from '@wdio/globals';
import { byId, launchHosted, openBoardOf, waitForCount } from '../lib/hosted.js';

const BRANCH = 'mission/e2e-stall';
/** The threshold of the run (one minute, lib/env.ts) plus two polls and the time to start. */
const STALL_WAIT_MS = 150_000;

describe('the stall banner of a hosted mission that waits on the human', () => {
  before(async () => {
    await launchHosted(BRANCH);
    await waitForCount('host-permission', 2);
    await openBoardOf(BRANCH);
  });

  it('shows no banner before the threshold', async () => {
    await expect(byId('stall-banner')).not.toBeExisting();
  });

  it('names the waiting prompt and the runbook section once the threshold passes', async () => {
    await byId('stall-banner').waitForDisplayed({
      timeout: STALL_WAIT_MS,
      timeoutMsg: 'no stall banner after the threshold',
    });
    await expect(byId('stall-prompt')).toHaveText('Waiting on human: answer it');
    await expect(byId('stall-link')).toHaveText(
      'docs/runbooks/stalled-mission.md#waiting-on-human',
    );
  });
});
