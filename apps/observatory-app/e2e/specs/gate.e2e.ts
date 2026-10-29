import { join } from 'node:path';
import { $, browser, expect } from '@wdio/globals';
import type { ChainablePromiseElement } from 'webdriverio';
import { REPORT_DIR, WAIT_MS } from '../lib/env.js';
import { byId, launchHosted, openBoardOf } from '../lib/hosted.js';
import { setTheme } from '../lib/theme.js';

const BRANCH = 'mission/e2e-gate';
const CARD = 'card-e2e-gate';
/** Lane ids and numbers of the default engineering template (config/cycles/engineering.toml). */
const BUILD = { id: 'build', number: 3 };
const GATE = { id: 'gate', number: 7 };

const cardIn = (lane: string) => $(`section[data-lane="${lane}"] [aria-label="${CARD}"]`);

/** Feeds the window one scenario of the pull request through the poll path of the shell. */
async function poll(scenario: string): Promise<void> {
  const answer = await browser.executeAsync((name: string, done: (value: string) => void) => {
    (window as any).__TAURI__.core
      .invoke('e2e_gate_poll', { scenario_name: name })
      .then((mission: string) => done(`ok ${mission}`), (error: unknown) => done(`error ${error}`));
  }, scenario);
  expect(answer).toMatch(/^ok /);
}

/** The named PNG the contract reads, next to the per-case ones the harness saves; `target` is scrolled into view first. */
async function png(name: string, target: ChainablePromiseElement): Promise<void> {
  try {
    await browser.execute((element: HTMLElement) => element.scrollIntoView({ block: 'center' }), await target);
    await browser.saveScreenshot(join(REPORT_DIR, `${name}.png`));
  } catch (error) {
    console.log(`screenshot ${name} failed: ${error}`);
  }
}

describe('the Gate lane of a hosted mission after the pull request polls', () => {
  before(async () => {
    await launchHosted(BRANCH);
    await openBoardOf(BRANCH);
  });

  it('red check moves the affected card to the build lane with cycle 1', async () => {
    await poll('red');
    await cardIn(BUILD.id).waitForDisplayed({ timeout: WAIT_MS, timeoutMsg: 'the card never reached the build lane' });
    await expect($(`section[aria-label="lane-${BUILD.number}"] [aria-label="${CARD}"]`)).toBeDisplayed();
    await expect(cardIn(GATE.id)).not.toBeExisting();
    await expect($(`[aria-label="${CARD}"] [data-testid="card-cycle"]`)).toHaveText('cycle 1');
    await expect(byId('gate-badge')).toHaveText('CI red');
    await png('card-lane3-cycle-1', cardIn(BUILD.id));
  });

  it('red check draws the return arc with cycle 1', async () => {
    const arc = $(`[aria-label="${CARD}"] [data-testid="return-arc"]`);
    await expect(arc).toBeDisplayed();
    await expect(arc).toHaveAttribute('data-from', GATE.id);
    await expect(arc).toHaveAttribute('data-to', BUILD.id);
    await expect(arc).toHaveText(expect.stringContaining('Gate → Build'));
    await expect(arc).toHaveText(expect.stringContaining('cycle 1'));
    await png('return-arc-cycle-1', arc);
  });

  it('the gate group lists the required check, reviews, ruleset and attestations', async () => {
    await expect(byId('gate-check-conclusion')).toHaveText('failed');
    await expect($('[data-testid="gate-check"]')).toHaveText(expect.stringContaining('demo-gate'));
    await expect(byId('gate-reviews')).toHaveText('reviews: 0');
    await expect(byId('gate-ruleset')).toHaveText('ruleset: demo-gate');
    await expect(byId('gate-attestations')).toHaveText('attestations: 0');
    await expect(byId('gate-clock')).toBeExisting();
  });

  it('merge button appears only when the checks are green and a human is required', async () => {
    await expect(byId('merge')).not.toBeExisting();
    await poll('green');
    await byId('merge').waitForDisplayed({ timeout: WAIT_MS, timeoutMsg: 'no Merge button after the green check' });
    await expect(byId('merge')).toHaveText('Merge');
    await expect(byId('gate-badge')).toHaveText('CI green');
    await cardIn(GATE.id).waitForDisplayed({ timeout: WAIT_MS });
    await expect($(`[aria-label="${CARD}"] [data-testid="card-cycle"]`)).toHaveText('cycle 1');
    await png('merge-button', byId('merge'));
  });

  it('a merged pull request reads handed to CD with the sha, the PR and the merger', async () => {
    await poll('merged');
    await browser.waitUntil(async () => (await byId('gate-badge').getText()) === 'handed to CD', {
      timeout: WAIT_MS,
      timeoutMsg: 'the badge never read handed to CD',
    });
    await expect(byId('gate-handed-to-cd')).toHaveText(
      expect.stringContaining('handed to CD: a3ec578 · PR #24'),
    );
    await expect(byId('gate-handed-to-cd')).toHaveText(expect.stringContaining('merged by human'));
    await expect(byId('merge')).not.toBeExisting();
  });

  it('the theme switch marks the active choice', async () => {
    await setTheme('dark');
    await expect(byId('theme-dark')).toHaveElementClass('on');
    await expect(byId('theme-light')).not.toHaveElementClass('on');
  });
});
