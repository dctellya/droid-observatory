import { readFileSync } from 'node:fs';
import { browser, expect } from '@wdio/globals';
import { FAKE_DROID_LOG, WAIT_MS } from '../lib/env.js';
import { allById, byId, hasRowKind, launchHosted, openBoardOf, waitForCount } from '../lib/hosted.js';
import { pageBackground, setTheme } from '../lib/theme.js';

const BRANCH = 'mission/e2e-controls';

const hostWrote = (): Record<string, any>[] =>
  readFileSync(FAKE_DROID_LOG, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((text) => JSON.parse(text));

async function waitForKind(kind: string): Promise<void> {
  await browser.waitUntil(() => hasRowKind(kind), {
    timeout: WAIT_MS,
    timeoutMsg: `no ${kind} row on the board`,
  });
}

describe('the control bar of a hosted mission on its board', () => {
  before(async () => {
    await launchHosted(BRANCH);
    await waitForCount('host-permission', 2);
    await openBoardOf(BRANCH);
    await byId('control-bar').waitForDisplayed({ timeout: WAIT_MS });
  });

  it('starts running with the questions and permission requests counted', async () => {
    await expect(byId('control-state')).toHaveText('running');
    await expect(byId('control-question-count')).toHaveText('1');
    await expect(byId('control-permission-count')).toHaveText('2');
    await expect(byId('control-resume')).toBeDisabled();
  });

  it('Pause records the event, the host acknowledges it and the bar shows paused', async () => {
    await byId('control-pause').click();
    await browser.waitUntil(async () => (await byId('control-state').getText()) === 'paused', {
      timeout: WAIT_MS,
      timeoutMsg: 'the bar never showed paused',
    });
    await expect(byId('control-result')).toHaveText(expect.stringContaining('action '));
    await expect(byId('control-pause')).toBeDisabled();
    await waitForKind('mission.paused');
  });

  it('Resume brings the bar back to running with a mission.resumed row', async () => {
    await byId('control-resume').click();
    await browser.waitUntil(async () => (await byId('control-state').getText()) === 'running', {
      timeout: WAIT_MS,
      timeoutMsg: 'the bar never showed running',
    });
    await waitForKind('mission.resumed');
  });

  it('Message writes the text to the host and records an approval.given row', async () => {
    await byId('control-message-text').setValue('use the staging table');
    await byId('control-message').click();
    await browser.waitUntil(
      async () => JSON.stringify(hostWrote()).includes('use the staging table'),
      { timeout: WAIT_MS, timeoutMsg: 'the host never wrote the message' },
    );
    await waitForKind('approval.given');
  });

  it('Approve and Deny close their requests and the host acknowledgements show as rows', async () => {
    const first = (await allById('host-permission'))[0];
    await first.$('[data-testid="host-approve"]').click();
    await waitForCount('host-permission', 1);
    await waitForKind('permission.granted');
    const second = (await allById('host-permission'))[0];
    await second.$('[data-testid="host-deny"]').click();
    await waitForCount('host-permission', 0);
    await waitForKind('permission.denied');
  });

  it('the dark theme paints the control bar and the AskUser sheet on the kit surfaces', async () => {
    const light = await pageBackground();
    await setTheme('dark');
    await expect(byId('control-bar')).toBeDisplayed();
    await expect(byId('ask-user-sheet')).toBeDisplayed();
    expect(await pageBackground()).not.toBe(light);
  });

  it('the light theme paints the control bar and the AskUser sheet on the kit surfaces', async () => {
    await setTheme('light');
    await expect(byId('control-bar')).toBeDisplayed();
    await expect(byId('ask-user-sheet')).toBeDisplayed();
  });

  it('the AskUser sheet answers the question and an intent.answered row follows', async () => {
    await expect(byId('ask-user-sheet')).toBeDisplayed();
    await byId('host-answer-text').setValue('the first');
    await byId('host-answer').click();
    await waitForCount('ask-user-sheet', 0);
    await waitForKind('question.answered');
    await waitForKind('intent.answered');
  });
});
