import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { FIXTURE_ROOT, REPORT_DIR, RUN_DIR } from './env.js';

/** A session id of its own, so the attached copy never shares event ids with the panel spec's replay. */
export const ATTACHED_ID = '5e2e0000-0000-4000-8000-00000000a77a';

const PROJECT_DIR = '-home-user-e2e-attach';
const STORE = join(RUN_DIR, 'observatory.sqlite');

/**
 * Copies the S fixture root session and its sidecar under the app's `[ingest].factory_sessions_dir`
 * (RUN_DIR/sessions, lib/env.ts) with the session id replaced. The file is new and fresh, so the
 * running watcher attaches it within a poll interval, like a session Droid has just started.
 */
export function seedAttachedSession(): string {
  const dir = join(RUN_DIR, 'sessions', PROJECT_DIR);
  mkdirSync(dir, { recursive: true });
  const original = JSON.parse(readFileSync(FIXTURE_ROOT, 'utf8').split('\n')[0]).id as string;
  const copy = (from: string, to: string) =>
    writeFileSync(to, readFileSync(from, 'utf8').replaceAll(original, ATTACHED_ID));
  copy(FIXTURE_ROOT.replace(/\.jsonl$/, '.settings.json'), join(dir, `${ATTACHED_ID}.settings.json`));
  const session = join(dir, `${ATTACHED_ID}.jsonl`);
  copy(FIXTURE_ROOT, session);
  return session;
}

/** Rows of a read-only query over the app's store, one `|`-separated string per row. */
export function query(sql: string): string[] {
  const run = spawnSync('sqlite3', ['-readonly', '-separator', '|', STORE, sql], { encoding: 'utf8' });
  if (run.status !== 0) throw new Error(`sqlite3 failed: ${run.stderr}`);
  return run.stdout.split('\n').filter(Boolean);
}

/** The app's stdout, which the tauri service captures into the newest wdio-*.log of the report directory. */
export function appLog(): string[] {
  const logs = readdirSync(REPORT_DIR)
    .filter((name) => /^wdio-.+\.log$/.test(name))
    .map((name) => join(REPORT_DIR, name))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  return logs.length === 0 ? [] : readFileSync(logs[0], 'utf8').split('\n');
}

/** Lines of the app log that show a JSON-RPC request written to a hosted process. */
export const rpcLines = (log: string[]): string[] => log.filter((line) => line.includes('host_control'));
