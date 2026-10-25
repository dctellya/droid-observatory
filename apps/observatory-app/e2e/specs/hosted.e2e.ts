import { readFileSync } from 'node:fs';
import { $, $$, browser, expect } from '@wdio/globals';
import {
  FAKE_DROID_GAP_MS,
  FAKE_DROID_LOG,
  HOSTED_MISSION_FILE,
  HOSTED_REPO,
  TIMING_LOG,
  WAIT_MS,
} from '../lib/env.js';
import { describe as describeSummary, summarize } from '../lib/stats.js';
import { readThresholds } from '../lib/thresholds.js';
import { readTimingLog, windowMeasures } from '../lib/timing.js';

const byId = (id: string) => $(`[data-testid="${id}"]`);
const allById = (id: string) => $$(`[data-testid="${id}"]`);

/** Lines the host wrote to the fake Droid's stdin. */
function hostWrote(): Record<string, any>[] {
  return readFileSync(FAKE_DROID_LOG, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((text) => JSON.parse(text));
}

async function count(id: string): Promise<number> {
  return (await allById(id)).length;
}

async function waitForCount(id: string, wanted: number): Promise<void> {
  await browser.waitUntil(async () => (await count(id)) === wanted, {
    timeout: WAIT_MS,
    timeoutMsg: `expected ${wanted} of ${id}`,
  });
}

/** Milliseconds since UTC midnight of a `HH:MM:SS.mmm` row time. */
function msOfDay(text: string): number {
  const [h, m, rest] = text.split(':');
  return (Number(h) * 60 + Number(m)) * 60_000 + Math.round(Number(rest) * 1000);
}

function utcMsOfDay(epoch: number): number {
  const day = 86_400_000;
  return ((epoch % day) + day) % day;
}

describe('a hosted mission driven through the window', () => {
  let startedAt = 0;

  before(async () => {
    // The app outlives a spec file: start from the library with a clean page.
    await browser.refresh();
    await byId('open-launch').waitForDisplayed({ timeout: WAIT_MS });
    await byId('open-launch').click();
    await byId('launch-field-repo').setValue(HOSTED_REPO);
    await byId('launch-field-mission-file').setValue(HOSTED_MISSION_FILE);
    await byId('launch-field-mission-branch-name').setValue('mission/e2e-fake');
    startedAt = Date.now();
    await byId('launch').click();
    await byId('hosted-run').waitForDisplayed({ timeout: WAIT_MS });
  });

  it('lists both permission requests and the question of the hosted run', async () => {
    await waitForCount('host-permission', 2);
    await waitForCount('ask-user-sheet', 1);
    await expect(byId('hosted-state')).toHaveText('running');
  });

  it('Approve removes the request and the host writes proceed_once to its id', async () => {
    const first = (await allById('host-permission'))[0];
    await expect(first).toHaveAttribute('data-request-id', 'perm-1');
    await first.$('[data-testid="host-approve"]').click();
    await waitForCount('host-permission', 1);
    await browser.waitUntil(async () => hostWrote().some((line) => line.id === 'perm-1'), {
      timeout: WAIT_MS,
    });
    const written = hostWrote().find((line) => line.id === 'perm-1');
    expect(written?.result).toEqual({ selectedOption: 'proceed_once' });
  });

  it('Deny removes the second request and the host writes cancel to its id', async () => {
    const left = (await allById('host-permission'))[0];
    await expect(left).toHaveAttribute('data-request-id', 'perm-2');
    await left.$('[data-testid="host-deny"]').click();
    await waitForCount('host-permission', 0);
    await browser.waitUntil(async () => hostWrote().some((line) => line.id === 'perm-2'), {
      timeout: WAIT_MS,
    });
    const written = hostWrote().find((line) => line.id === 'perm-2');
    expect(written?.result).toEqual({ selectedOption: 'cancel' });
  });

  it('the AskUser sheet writes the typed answer to the droid.ask_user request id', async () => {
    const sheet = byId('ask-user-sheet');
    await expect(sheet).toBeDisplayed();
    await expect(sheet).toHaveText(expect.stringContaining('Which one?'));
    await byId('host-answer-text').setValue('the first');
    await byId('host-answer').click();
    await waitForCount('ask-user-sheet', 0);
    await browser.waitUntil(async () => hostWrote().some((line) => line.id === 'req-ask'), {
      timeout: WAIT_MS,
    });
    const written = hostWrote().find((line) => line.id === 'req-ask');
    expect(written?.result.answers[0].answer).toBe('the first');
  });

  describe('on the board of the hosted mission', () => {
    before(async () => {
      await byId('launch-back').click();
      const row = await browser.waitUntil(
        async () => {
          for (const candidate of await allById('library-row')) {
            if ((await candidate.getText()).includes('e2e-fake')) return candidate;
          }
          return false;
        },
        { timeout: WAIT_MS, timeoutMsg: 'the hosted mission is not in the library' },
      );
      await row.$('[data-testid="library-open"]').click();
      await byId('board').waitForDisplayed({ timeout: WAIT_MS });
    });

    it('an intent.answered row follows the answer', async () => {
      await browser.waitUntil(
        async () => {
          for (const row of await allById('row-kind')) {
            if ((await row.getText()) === 'intent.answered') return true;
          }
          return false;
        },
        { timeout: WAIT_MS, timeoutMsg: 'no intent.answered row on the board' },
      );
    });

    it('stamps each event with the time its line arrived, one poll apart', async () => {
      const finishedAt = Date.now();
      const stamped = new Map<string, { kind: string; at: number }>();
      for (const row of await allById('timeline-row')) {
        const id = await row.getAttribute('data-event-id');
        const kind = await row.$('[data-testid="row-kind"]').getText();
        const time = await row.$('[data-testid="row-time"]').getText();
        stamped.set(id ?? '', { kind, at: msOfDay(time) });
      }
      const lower = utcMsOfDay(startedAt);
      const upper = utcMsOfDay(finishedAt);
      for (const row of stamped.values()) {
        expect(row.at).toBeGreaterThanOrEqual(lower);
        expect(row.at).toBeLessThanOrEqual(upper);
      }
      const failures = [...stamped.values()]
        .filter((row) => row.kind === 'tool.error')
        .map((row) => row.at)
        .sort((a, b) => a - b);
      expect(failures.length).toBeGreaterThanOrEqual(2);
      const gap = failures[1] - failures[0];
      console.log(`tool.error arrival stamps ${failures.join(', ')} ms of day, first gap ${gap} ms`);
      expect(gap).toBeGreaterThanOrEqual(FAKE_DROID_GAP_MS / 2);
    });
  });

  it('shows each event within the configured latency, from User Timing and the host log', async () => {
    const limits = readThresholds();
    const window = summarize(await windowMeasures('event:first_on_board'));
    const host = readTimingLog(TIMING_LOG).filter((line) => line.interval === 'spool_to_appended');
    const hostSummary = summarize(host.map((line) => line.ms));
    console.log(describeSummary('event:first_on_board (window)', window));
    console.log(describeSummary('spool_to_appended (host)', hostSummary));
    expect(window.samples).toBeGreaterThanOrEqual(limits.min_samples);
    expect(hostSummary.samples).toBeGreaterThanOrEqual(limits.min_samples);
    expect(window.p95).toBeLessThanOrEqual(limits.event_first_on_board_p95_ms);
    expect(hostSummary.p95).toBeLessThanOrEqual(limits.host_append_p95_ms);
    expect(window.max + hostSummary.max).toBeLessThanOrEqual(limits.event_visible_max_ms);
  });
});
