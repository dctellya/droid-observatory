import { spawnSync } from 'node:child_process';
import { REPO_ROOT } from './env.js';

export interface ReplayCounts {
  /** Events per lane number (the `N` of the `N Name` column). */
  lanes: Map<number, number>;
  total: number;
  unknown: number;
}

/** What `observatory-cli replay` prints for a capture: the oracle the window is compared with. */
export function cliReplay(root: string): ReplayCounts {
  const run = spawnSync('cargo', ['run', '-q', '-p', 'observatory-cli', '--', 'replay', root], {
    cwd: REPO_ROOT,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  if (run.status !== 0) throw new Error(`observatory-cli replay failed: ${run.stderr}`);
  const lanes = new Map<number, number>();
  let total = 0;
  let unknown = -1;
  for (const line of run.stdout.split('\n')) {
    const tail = /^unknown events: (\d+)$/.exec(line);
    if (tail) {
      unknown = Number(tail[1]);
      continue;
    }
    const lane = /^[^\t]+\t(\d+) [^\t]+\t/.exec(line);
    if (lane) {
      const number = Number(lane[1]);
      lanes.set(number, (lanes.get(number) ?? 0) + 1);
      total += 1;
    }
  }
  if (unknown < 0) throw new Error('the replay printed no `unknown events:` line');
  return { lanes, total, unknown };
}
