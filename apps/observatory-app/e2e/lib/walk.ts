import { spawnSync } from 'node:child_process';
import { appendFileSync, existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { browser } from '@wdio/globals';
import type { ChainablePromiseElement } from 'webdriverio';
import { $$ } from '@wdio/globals';
import { ATTACHED_ID, appLog, seedAttachedSession } from './attach.js';
import { WAIT_MS } from './env.js';
import { byId } from './hosted.js';
import { REPORT_DIR, RUN_DIR } from './env.js';

/** The repository and branches of the demo pair the live walk drives (ai/ demo workflow `demo-gate`). */
export const WALK = { repo: 'dctellya/droid-observatory', base: 'demo/base', head: 'demo/red-1' };

const COMMANDS = () => join(REPORT_DIR, 'walk-commands.log');
const PR_FILE = () => '/tmp/m4-walk-pr.txt';

/** Runs a command and keeps it, its exit code and its output in the walk's command log. */
export function sh(command: string, ...args: string[]): string {
  const run = spawnSync(command, args, { encoding: 'utf8' });
  appendFileSync(
    COMMANDS(),
    `${new Date().toISOString()} $ ${command} ${args.join(' ')}\n[exit ${run.status}]\n${run.stdout}${run.stderr}\n`,
  );
  if (run.status !== 0) throw new Error(`${command} ${args.join(' ')} failed: ${run.stderr}`);
  return run.stdout.trim();
}

export const gh = (...args: string[]) => sh('gh', ...args);

export const savePr = (number: number) => writeFileSync(PR_FILE(), String(number));
export const savedPr = (): number => {
  if (!existsSync(PR_FILE())) throw new Error('no pull request recorded by the first walk run');
  return Number(readFileSync(PR_FILE(), 'utf8'));
};

/** Every run of `demo-gate` on the head sha of the pull request: id, status, conclusion, event. */
export function runsOf(sha: string): string {
  return gh(
    'api',
    `repos/${WALK.repo}/actions/runs?head_sha=${sha}`,
    '--jq',
    '.workflow_runs[] | "run \\(.id) \\(.event) \\(.status) \\(.conclusion)"',
  );
}

/** A named PNG of the window, `target` scrolled into view first. */
export async function png(name: string, target: ChainablePromiseElement): Promise<void> {
  await browser.execute((element: HTMLElement) => element.scrollIntoView({ block: 'center' }), await target);
  await browser.saveScreenshot(join(REPORT_DIR, `${name}.png`));
}

/** Lines of the app log that carry one of the given event names, kept in the walk's command log. */
export function keepLog(names: string[]): string[] {
  const lines = appLog().filter((line) => names.some((name) => line.includes(name)));
  appendFileSync(COMMANDS(), `${new Date().toISOString()} app log ${names.join(',')}\n${lines.join('\n')}\n\n`);
  return lines;
}

/**
 * Attaches the S fixture session (a mission with milestones, so a fix cycle has a card to return) and opens its
 * board; the walk's config names the demo head branch as the mission's branch.
 */
export async function openAttached(): Promise<void> {
  await browser.refresh();
  await byId('open-launch').waitForDisplayed({ timeout: WAIT_MS });
  seedAttachedSession();
  const open = await browser.waitUntil(
    async () => {
      for (const row of await $$('[aria-label="watched-session"]')) {
        if ((await row.getText()).includes(ATTACHED_ID)) return row.$('[aria-label="attach-session"]');
      }
      return false;
    },
    { timeout: WAIT_MS, timeoutMsg: 'the watcher never attached the copied session' },
  );
  await open.click();
  await byId('board').waitForDisplayed({ timeout: WAIT_MS });
}
