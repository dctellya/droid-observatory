import { $, browser, expect } from '@wdio/globals';
import { query } from '../lib/attach.js';
import { RUN_DIR } from '../lib/env.js';
import { startForgeProxy, type ForgeStub } from '../lib/forge.js';
import { byId } from '../lib/hosted.js';
import { gh, openAttached, keepLog, png, runsOf, savePr, WALK } from '../lib/walk.js';

const WAIT = 240_000;

describe('live walk 1: demo/red-1 opens red, the label comes off, the check goes green', () => {
  let forge: ForgeStub;
  let pr = 0;
  let sha = '';

  before(async () => {
    forge = await startForgeProxy();
    await openAttached();
  });
  after(async () => {
    await forge.close();
  });

  it('opens the pull request with the label demo:red and the badge reads CI red', async () => {
    const base = gh('api', `repos/${WALK.repo}/git/ref/heads/${WALK.base}`, '--jq', '.object.sha');
    const tree = gh('api', `repos/${WALK.repo}/git/commits/${base}`, '--jq', '.tree.sha');
    sha = gh('api', `repos/${WALK.repo}/git/commits`, '-f', 'message=demo: red-1 live walk', '-f', `tree=${tree}`, '-f', `parents[]=${base}`, '--jq', '.sha');
    gh('api', `repos/${WALK.repo}/git/refs`, '-f', `ref=refs/heads/${WALK.head}`, '-f', `sha=${sha}`);
    const url = gh('pr', 'create', '--repo', WALK.repo, '--base', WALK.base, '--head', WALK.head, '--title', 'demo: red-1 live walk', '--body', 'Live walk of the Gate lane on the demo pair.', '--label', 'demo:red');
    pr = Number(url.split('/').pop());
    savePr(pr);
    await browser.waitUntil(async () => (await byId('gate-badge').isExisting()) && (await byId('gate-badge').getText()) === 'CI red', {
      timeout: WAIT,
      timeoutMsg: 'the badge never read CI red',
    });
    await $('[data-testid="card-cycle"]').waitForExist({ timeout: 60_000, timeoutMsg: 'no cycle count on the card' });
    await expect($('[data-testid="card-cycle"]')).toHaveText('cycle 1');
    await expect($('[data-testid="return-arc"]')).toBeDisplayed();
    keepLog(['gate_changed', 'github_poll']);
    gh('api', `repos/${WALK.repo}/actions/runs?head_sha=${sha}`, '--jq', '.workflow_runs[] | "run \\(.id) \\(.event) \\(.status) \\(.conclusion)"');
    await png('live-red-cycle-1', byId('gate-group'));
  });

  it('removing the label turns the badge green and offers Merge', async () => {
    gh('pr', 'edit', String(pr), '--repo', WALK.repo, '--remove-label', 'demo:red');
    await browser.waitUntil(async () => (await byId('gate-badge').getText()) === 'CI green', {
      timeout: WAIT,
      timeoutMsg: 'the badge never read CI green',
    });
    await byId('merge').waitForDisplayed({ timeout: 60_000, timeoutMsg: 'no Merge button after green' });
    await $('[data-testid="card-cycle"]').waitForExist({ timeout: 60_000 });
    await expect($('[data-testid="card-cycle"]')).toHaveText('cycle 1');
    keepLog(['gate_changed']);
    runsOf(sha);
    gh('pr', 'view', String(pr), '--repo', WALK.repo, '--json', 'number,headRefOid,state,labels');
    await png('live-green-merge', byId('gate-group'));
    expect(forge.requests.every((line) => line.startsWith('GET '))).toBe(true);
    expect(query('SELECT COUNT(*) FROM human_actions')).toEqual(['0']);
    void RUN_DIR;
  });
});
