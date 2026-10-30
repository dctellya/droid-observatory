import { join } from 'node:path';
import { $, browser, expect } from '@wdio/globals';
import type { ChainablePromiseElement } from 'webdriverio';
import { appLog, query } from '../lib/attach.js';
import { REPORT_DIR, WAIT_MS } from '../lib/env.js';
import { type ForgeStub, startForgeStub } from '../lib/forge.js';
import { byId, launchHosted, openBoardOf } from '../lib/hosted.js';
import { setTheme } from '../lib/theme.js';

const BRANCH = 'mission/e2e-gate';
const CARD = 'card-e2e-gate';
/** Lane ids and numbers of the default engineering template (config/cycles/engineering.toml). */
const BUILD = { id: 'build', number: 3 };
const GATE = { id: 'gate', number: 7 };

/** The worktree the e2e build caches for the mission's milestone (src-tauri/src/e2e_gate.rs). */
const WORKTREE = { name: 'wt-e2e-gate', path: '/e2e/worktrees/wt-e2e-gate' };

const PR = 24;
const REFUSED = `Merge of PR #${PR} into demo/base refused: token lacks pull_requests:write. Merge in GitHub; the badge updates from the next poll.`;

/** The lines of the app log that show the merge request of the PR leaving the app. */
const putLines = () => appLog().filter((line) => line.includes('github.request') && line.includes(`pulls/${PR}/merge`));

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
  let forge: ForgeStub;
  let firstRow = 0;

  before(async () => {
    forge = await startForgeStub();
    await launchHosted(BRANCH);
    await openBoardOf(BRANCH);
  });

  after(async () => {
    await forge.close();
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

  it('the card names the worktree git lists for its milestone, the path as its title', async () => {
    const name = $(`[aria-label="${CARD}"] [data-testid="card-worktree"]`);
    await name.waitForDisplayed({ timeout: WAIT_MS, timeoutMsg: 'the card never named its worktree' });
    await expect(name).toHaveText(WORKTREE.name);
    await expect(name).toHaveAttribute('title', WORKTREE.path);
    await png('card-worktree-name', name);
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

  it('merge click opens a confirmation naming the PR and the target', async () => {
    firstRow = Number(query('SELECT COALESCE(MAX(id), 0) FROM human_actions')[0]);
    await byId('merge').click();
    const dialog = byId('merge-confirmation');
    await dialog.waitForDisplayed({ timeout: WAIT_MS, timeoutMsg: 'no confirmation after the click' });
    await expect(byId('merge-confirmation-text')).toHaveText(expect.stringContaining(`PR #${PR}`));
    await expect(byId('merge-confirmation-text')).toHaveText(expect.stringContaining('demo/base'));
    expect(forge.requests).toEqual([]);
    expect(putLines()).toEqual([]);
    await png('confirm-dialog', dialog);
  });

  it('cancelling the confirmation sends nothing and keeps the button', async () => {
    await byId('merge-cancel').click();
    await expect(byId('merge-confirmation')).not.toBeExisting();
    await expect(byId('merge')).toBeDisplayed();
    expect(forge.requests).toEqual([]);
    expect(query(`SELECT COUNT(*) FROM human_actions WHERE kind = 'merge' AND id > ${firstRow}`)).toEqual(['0']);
  });

  it('confirming the merge with the read-only token shows the refusal message', async () => {
    await byId('merge').click();
    await byId('merge-confirm').click();
    const result = byId('merge-result');
    await result.waitForDisplayed({ timeout: WAIT_MS, timeoutMsg: 'no answer after the confirmation' });
    await expect(result).toHaveText(REFUSED);
    await expect(byId('merge')).toBeDisplayed();
    await expect(byId('merge-confirmation')).not.toBeExisting();
    expect(forge.requests).toEqual([`PUT /repos/dctellya/droid-observatory/pulls/${PR}/merge`]);
    await browser.waitUntil(() => putLines().length === 1, { timeout: WAIT_MS, timeoutMsg: 'no PUT line in the app log' });
    expect(putLines()[0]).toContain('status=403');
    await png('403-message', result);
  });

  it('two presses of Merge after the 403 send two requests', async () => {
    await byId('merge').click();
    await byId('merge-confirm').click();
    await browser.waitUntil(() => forge.requests.length === 2, { timeout: WAIT_MS, timeoutMsg: 'no second request' });
    await browser.waitUntil(async () => (await byId('merge').isEnabled()) === true, { timeout: WAIT_MS });
    await expect(byId('merge-result')).toHaveText(REFUSED);
    await browser.waitUntil(() => putLines().length === 2, { timeout: WAIT_MS, timeoutMsg: 'no second PUT line in the app log' });
    expect(putLines().every((line) => line.includes('status=403'))).toBe(true);
    expect(
      query(`SELECT COUNT(*) FROM human_actions WHERE kind = 'merge' AND state = 'refused' AND id > ${firstRow}`),
    ).toEqual(['2']);
    // Each press is one human-overlay approval.given event on the Gate lane (the store keeps lane ids since migration 0002).
    expect(
      query(
        `SELECT COUNT(*) FROM events e JOIN human_actions a ON a.id = e.action_id WHERE e.kind = 'approval.given' ` +
          `AND e.overlay = 'human' AND e.role = 'human' AND e.source = 'app' AND e.lane = '${GATE.id}' ` +
          `AND a.kind = 'merge' AND a.id > ${firstRow}`,
      ),
    ).toEqual(['2']);
    await png('two-presses-refused', byId('gate-group'));
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
