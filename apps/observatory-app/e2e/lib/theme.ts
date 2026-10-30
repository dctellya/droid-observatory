import { browser, expect } from '@wdio/globals';
import { byId } from './hosted.js';

/** Presses the kit's theme switch and checks `data-theme` on the root, so the case's PNG shows that theme. */
export async function setTheme(theme: 'dark' | 'light'): Promise<void> {
  await byId(`theme-${theme}`).click();
  const applied = await browser.execute(() => document.documentElement.dataset.theme);
  expect(applied).toBe(theme);
}

/** The computed page background, which the kit's tokens change with the theme. */
export async function pageBackground(): Promise<string> {
  return browser.execute(() => getComputedStyle(document.body).backgroundColor);
}
