import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

export const E2E_DIR = resolve(here, '..');
export const REPO_ROOT = resolve(E2E_DIR, '../../..');
export const DATE = process.env.E2E_DATE ?? new Date().toISOString().slice(0, 10);
export const REPORT_DIR = process.env.E2E_REPORT_DIR ?? join(REPO_ROOT, 'reports', 'e2e', DATE);
export const RUN_DIR = process.env.E2E_RUN_DIR ?? join(tmpdir(), 'observatory-e2e');
export const TIMING_LOG = join(REPORT_DIR, 'timing.jsonl');
export const FAKE_DROID_LOG = join(RUN_DIR, 'fake-droid-stdin.jsonl');
export const FAKE_DROID_GAP_MS = 400;
export const APP_BINARY = join(
  process.env.CARGO_TARGET_DIR ?? join(REPO_ROOT, 'target'),
  'debug',
  'observatory-app',
);
export const FIXTURE_ROOT = join(REPO_ROOT, 'fixtures/streams/2026-10-22-observatory-v1-s.jsonl');
export const HOSTED_REPO = join(RUN_DIR, 'repo');
export const HOSTED_MISSION_FILE = join(RUN_DIR, 'mission.md');

/** Port of the loopback forge the gate spec starts (lib/forge.ts); 3142-3149 are the mission's scratch ports. */
export const FORGE_STUB_PORT = 3147;

/** How long a scenario waits for something the app does on its own. */
export const WAIT_MS = 30_000;

/**
 * Everything the app reads is under RUN_DIR: its own config, store and spool, and a Droid that is
 * the fake one. The files are rewritten with the same content on every call, so the launcher and
 * each worker may call it.
 */
export function prepareRunDir(): void {
  for (const dir of [RUN_DIR, REPORT_DIR, HOSTED_REPO, join(RUN_DIR, 'sessions'), join(RUN_DIR, 'logs')]) {
    mkdirSync(dir, { recursive: true });
  }
  writeFileSync(HOSTED_MISSION_FILE, '# E2E mission\n\nNothing to build.\n');
  const toml = [
    '[mission]',
    `droid_bin = "${join(E2E_DIR, 'fixtures/fake-droid.mjs')}"`,
    `spool_base = "${join(RUN_DIR, 'spool')}"`,
    'reap_grace_ms = 500',
    '',
    '[ingest]',
    `factory_sessions_dir = "${join(RUN_DIR, 'sessions')}"`,
    `factory_logs_dir = "${join(RUN_DIR, 'logs')}"`,
    '',
    '[github]',
    // The gate spec feeds the window its own polls; a real poll of the repository would race them.
    'poll_interval_s = 3600',
    `api_base = "http://127.0.0.1:${FORGE_STUB_PORT}"`,
    '',
    '[intervention]',
    'stall_threshold_min = 1',
    'stall_poll_s = 5',
    '',
    '[store]',
    `path = "${join(RUN_DIR, 'observatory.sqlite')}"`,
    '',
  ].join('\n');
  // A live walk of the gate (a real poll of the demo pair) brings its own config; `{RUN_DIR}` stands for RUN_DIR.
  const own = process.env.E2E_CONFIG_FILE;
  const config = own ? readFileSync(own, 'utf8').replaceAll('{RUN_DIR}', RUN_DIR) : toml;
  writeFileSync(join(RUN_DIR, 'config.toml'), config);
}

/**
 * A PAT-shaped value built at run time (a literal trips secret scanners). The app reads the keychain
 * only under FORGE_LIVE=1; a launched app has a token in its environment so no worker-started
 * process can prompt the owner.
 */
export function fakeGithubToken(): string {
  return ['github', 'pat', 'x'.repeat(40)].join('_');
}

export function appEnv(): Record<string, string> {
  return {
    GITHUB_TOKEN: process.env.GITHUB_TOKEN || fakeGithubToken(),
    OBSERVATORY_CONFIG: join(RUN_DIR, 'config.toml'),
    OBSERVATORY_TIMING_LOG: TIMING_LOG,
    FAKE_DROID_LOG,
    FAKE_DROID_GAP_MS: String(FAKE_DROID_GAP_MS),
  };
}
