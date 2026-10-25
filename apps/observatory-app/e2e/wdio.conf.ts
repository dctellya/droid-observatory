import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { APP_BINARY, DATE, REPORT_DIR, appEnv, prepareRunDir } from './lib/env.js';

// The app starts with this process' environment plus `env` below; the files it names are written
// before any worker starts and again, unchanged, by each worker that loads this file.
prepareRunDir();

const slug = (title: string) => title.replace(/[^A-Za-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 80);

export const config: WebdriverIO.Config = {
  runner: 'local',
  specs: ['./specs/**/*.e2e.ts'],
  // One app instance at a time: they share one store, one timing log and one embedded-driver port.
  maxInstances: 1,
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 120_000 },
  capabilities: [{ browserName: 'tauri' }],
  services: [
    [
      'tauri',
      {
        appBinaryPath: APP_BINARY,
        driverProvider: 'embedded',
        embeddedPort: 4445,
        startTimeout: 60_000,
        captureBackendLogs: true,
        env: appEnv(),
      },
    ],
  ],
  reporters: [
    'spec',
    [
      'junit',
      {
        outputDir: REPORT_DIR,
        outputFileFormat: ({ cid }: { cid: string }) => `junit-${cid}.xml`,
        // The p50/p95/max lines the specs print land in each case's system-out.
        addWorkerLogs: true,
      },
    ],
  ],
  logLevel: 'warn',
  outputDir: REPORT_DIR,
  onPrepare: () => {
    mkdirSync(REPORT_DIR, { recursive: true });
    console.log(`e2e ${DATE}: app ${APP_BINARY}, reports ${REPORT_DIR}`);
  },
  onComplete: () => {
    // One file per spec file is written by the reporter; the report is their <testsuite> elements together.
    const parts = readdirSync(REPORT_DIR).filter((f) => /^junit-.+\.xml$/.test(f)).sort();
    const suites = parts.flatMap((f) => [readFileSync(join(REPORT_DIR, f), 'utf8').match(/<testsuites[^>]*>([\s\S]*)<\/testsuites>/)?.[1]?.trim() ?? '']);
    writeFileSync(join(REPORT_DIR, 'junit.xml'), `<?xml version="1.0" encoding="UTF-8"?>\n<testsuites>\n${suites.join('\n')}\n</testsuites>\n`);
    for (const f of parts) rmSync(join(REPORT_DIR, f));
  },
  before: async () => {
    // The tauri service checks the window state before every find and click through
    // `browser.tauri.execute`, which needs tauri-plugin-wdio and costs 5 s per command without it.
    // An explicit switchToWindow turns that recovery off for the session.
    await browser.switchToWindow(await browser.getWindowHandle());
    console.log(`document.visibilityState = ${await browser.execute(() => document.visibilityState)}`);
  },
  afterTest: async (test, _context, { passed }) => {
    // A covered or locked window may return a blank image or refuse; the DOM asserts stand alone.
    try {
      await browser.saveScreenshot(join(REPORT_DIR, `${passed ? 'pass' : 'fail'}-${slug(test.title)}.png`));
    } catch (error) {
      console.log(`screenshot of "${test.title}" failed: ${error}`);
    }
  },
};
