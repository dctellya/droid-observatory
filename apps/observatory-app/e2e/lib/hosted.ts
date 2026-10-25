import { $, $$, browser } from '@wdio/globals';
import { HOSTED_MISSION_FILE, HOSTED_REPO, WAIT_MS } from './env.js';

export const byId = (id: string) => $(`[data-testid="${id}"]`);
export const allById = (id: string) => $$(`[data-testid="${id}"]`);

export async function count(id: string): Promise<number> {
  return (await allById(id)).length;
}

export async function waitForCount(id: string, wanted: number): Promise<void> {
  await browser.waitUntil(async () => (await count(id)) === wanted, {
    timeout: WAIT_MS,
    timeoutMsg: `expected ${wanted} of ${id}`,
  });
}

/** Launches a hosted mission on the branch and waits for its run section. */
export async function launchHosted(branch: string): Promise<void> {
  await browser.refresh();
  await byId('open-launch').waitForDisplayed({ timeout: WAIT_MS });
  await byId('open-launch').click();
  await byId('launch-field-repo').setValue(HOSTED_REPO);
  await byId('launch-field-mission-file').setValue(HOSTED_MISSION_FILE);
  await byId('launch-field-mission-branch-name').setValue(branch);
  await byId('launch').click();
  await byId('hosted-run').waitForDisplayed({ timeout: WAIT_MS });
}

/** Back to the library and into the board of the hosted mission whose name has `branch`. */
export async function openBoardOf(branch: string): Promise<void> {
  await byId('launch-back').click();
  const row = await browser.waitUntil(
    async () => {
      for (const candidate of await allById('library-row')) {
        if ((await candidate.getText()).includes(branch)) return candidate;
      }
      return false;
    },
    { timeout: WAIT_MS, timeoutMsg: `${branch} is not in the library` },
  );
  await row.$('[data-testid="library-open"]').click();
  await byId('board').waitForDisplayed({ timeout: WAIT_MS });
}

/** True once a row of the lanes shows an event of this kind. */
export async function hasRowKind(kind: string): Promise<boolean> {
  for (const row of await allById('row-kind')) {
    if ((await row.getText()) === kind) return true;
  }
  return false;
}
