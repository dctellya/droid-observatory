import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { REPO_ROOT } from './env.js';

export interface E2eThresholds {
  panel_render_p95_ms: number;
  event_first_on_board_p95_ms: number;
  host_append_p95_ms: number;
  event_visible_max_ms: number;
  min_samples: number;
}

/** The `[e2e]` table of config/observatory.example.toml: `key = integer` lines, `#` comments. */
export function readThresholds(): E2eThresholds {
  const text = readFileSync(join(REPO_ROOT, 'config/observatory.example.toml'), 'utf8');
  const table: Record<string, number> = {};
  let inside = false;
  for (const raw of text.split('\n')) {
    const line = raw.replace(/#.*$/, '').trim();
    if (line.startsWith('[')) {
      inside = line === '[e2e]';
    } else if (inside && line.includes('=')) {
      const [key, value] = line.split('=').map((part) => part.trim());
      table[key] = Number(value);
    }
  }
  const wanted: (keyof E2eThresholds)[] = [
    'panel_render_p95_ms',
    'event_first_on_board_p95_ms',
    'host_append_p95_ms',
    'event_visible_max_ms',
    'min_samples',
  ];
  for (const key of wanted) {
    if (!Number.isFinite(table[key])) throw new Error(`[e2e] ${key} missing from the example config`);
  }
  return table as unknown as E2eThresholds;
}
