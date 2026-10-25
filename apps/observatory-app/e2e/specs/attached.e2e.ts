import { $$, browser, expect } from '@wdio/globals';
import { WAIT_MS } from '../lib/env.js';
import { ATTACHED_ID, appLog, query, rpcLines, seedAttachedSession } from '../lib/attach.js';
import { byId } from '../lib/hosted.js';

const TEXT = 'check the attached copy first';
const INSTRUCTION = 'copy to the terminal running the mission';

describe('the control bar of an attached mission, which has no control channel', () => {
  let rpcBefore = 0;

  before(async () => {
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
    rpcBefore = rpcLines(appLog()).length;
    await open.click();
    await byId('control-bar').waitForDisplayed({ timeout: WAIT_MS });
  });

  it('shows the terminal instruction under the bar', async () => {
    await expect(byId('terminal-instruction')).toHaveText(INSTRUCTION);
    await expect(byId('control-message')).toBeEnabled();
  });

  it('Message shows the text to paste and records a human overlay row without a host request', async () => {
    await byId('control-message-text').setValue(TEXT);
    await byId('control-message').click();
    await expect(byId('terminal-text')).toHaveText(expect.stringContaining(TEXT));
    await browser.waitUntil(
      () => query(`SELECT kind, state FROM human_actions WHERE mission_id = '${ATTACHED_ID}'`).length === 1,
      { timeout: WAIT_MS, timeoutMsg: 'no human_actions row for the attached mission' },
    );
    expect(query(`SELECT kind, state FROM human_actions WHERE mission_id = '${ATTACHED_ID}'`)).toEqual([
      'message|sent',
    ]);
    expect(
      query(
        `SELECT source, overlay, lane, role FROM events WHERE mission_id = '${ATTACHED_ID}' AND source = 'app'`,
      ),
    ).toEqual(['app|human|plan|human']);
  });

  it('writes a human_action line with text_len to the app log and no request to a process', async () => {
    const lines = await browser.waitUntil(
      () => {
        const found = appLog().filter(
          (line) => line.includes('human_action') && line.includes(`mission_id=${ATTACHED_ID}`),
        );
        return found.length > 0 && found;
      },
      { timeout: WAIT_MS, timeoutMsg: 'the app log has no human_action line for the attached mission' },
    );
    expect(lines[0]).toContain(`text_len: ${TEXT.length}`);
    const log = appLog();
    expect(log.filter((line) => line.includes(TEXT))).toEqual([]);
    expect(rpcLines(log).length).toBe(rpcBefore);
  });
});
