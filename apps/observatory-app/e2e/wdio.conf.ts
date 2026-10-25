import { mkdirSync } from 'node:fs';
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
        outputFileFormat: () => 'junit.xml',
      },
    ],
  ],
  logLevel: 'warn',
  outputDir: REPORT_DIR,
  onPrepare: () => {
    mkdirSync(REPORT_DIR, { recursive: true });
    console.log(`e2e ${DATE}: app ${APP_BINARY}, reports ${REPORT_DIR}`);
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
