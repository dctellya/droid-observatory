import { $, $$, browser, expect } from '@wdio/globals';
import { FIXTURE_ROOT, WAIT_MS } from '../lib/env.js';
import { cliReplay, type ReplayCounts } from '../lib/replay.js';
import { describe as describeSummary, summarize } from '../lib/stats.js';
import { readThresholds } from '../lib/thresholds.js';
import { windowMeasures } from '../lib/timing.js';

const byId = (id: string) => $(`[data-testid="${id}"]`);

async function openFixture(): Promise<void> {
  await byId('open-path').setValue(FIXTURE_ROOT);
  await byId('open-path-go').click();
  await byId('board').waitForDisplayed({ timeout: WAIT_MS });
}

async function reopenFromLibrary(): Promise<void> {
  await byId('back-to-library').click();
  await byId('open-path').waitForDisplayed({ timeout: WAIT_MS });
  await openFixture();
}

describe('panel render for a replayed fixture', () => {
  let oracle: ReplayCounts;

  before(async () => {
    oracle = cliReplay(FIXTURE_ROOT);
    // The app outlives a spec file: start from the library, with no measures from an earlier one.
    await browser.refresh();
    await byId('open-path').waitForDisplayed({ timeout: WAIT_MS });
    await openFixture();
  });

  it('draws every lane with the count the CLI replay prints', async () => {
    const drawn: number[] = [];
    for (const lane of await $$('[data-testid^="lane-count-"]')) {
      const id = await lane.getAttribute('data-testid');
      drawn.push(Number((id ?? '').replace('lane-count-', '')));
    }
    for (const number of oracle.lanes.keys()) expect(drawn).toContain(number);
    for (const number of drawn) {
      await expect(byId(`lane-count-${number}`)).toHaveText(String(oracle.lanes.get(number) ?? 0));
    }
    console.log(`lane counts (CLI replay): ${JSON.stringify([...oracle.lanes])}`);
  });

  it('shows the event total and the unknown-events count of the replay', async () => {
    await expect(byId('event-count')).toHaveText(String(oracle.total));
    await expect(byId('unknown-count')).toHaveText(String(oracle.unknown));
  });

  it('renders the Panel with its elapsed time and the role table', async () => {
    await expect(byId('panel')).toBeDisplayed();
    await expect(byId('panel-elapsed')).toHaveText(expect.stringMatching(/^Elapsed: (\d+ s|n\/a)$/));
    await expect(byId('panel-roles')).toBeDisplayed();
  });

  it('renders the Panel within the configured latency, measured by User Timing', async () => {
    const limits = readThresholds();
    await reopenFromLibrary();
    await reopenFromLibrary();
    const durations = await windowMeasures('panel:render');
    const summary = summarize(durations);
    console.log(describeSummary('panel:render', summary));
    expect(summary.samples).toBeGreaterThanOrEqual(limits.min_samples);
    expect(summary.p95).toBeLessThanOrEqual(limits.panel_render_p95_ms);
  });
});
