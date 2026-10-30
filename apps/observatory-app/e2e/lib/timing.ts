import { existsSync, readFileSync } from 'node:fs';

export interface TimingLine {
  interval: string;
  ms: number;
  count?: number;
}

/** The JSON lines the host wrote to OBSERVATORY_TIMING_LOG (target `observatory::timing`). */
export function readTimingLog(path: string): TimingLine[] {
  if (!existsSync(path)) return [];
  const lines: TimingLine[] = [];
  for (const text of readFileSync(path, 'utf8').split('\n')) {
    if (!text.trim()) continue;
    const parsed = JSON.parse(text);
    if (parsed.target !== 'observatory::timing') continue;
    lines.push({
      interval: String(parsed.fields.interval),
      ms: Number(parsed.fields.ms),
      count: parsed.fields.count === undefined ? undefined : Number(parsed.fields.count),
    });
  }
  return lines;
}

export interface Measure {
  name: string;
  duration: number;
}

/** The User Timing measures the window recorded under `name`. */
export async function windowMeasures(name: string): Promise<number[]> {
  const entries = (await browser.execute(() =>
    performance
      .getEntriesByType('measure')
      .map((entry) => ({ name: entry.name, duration: entry.duration })),
  )) as Measure[];
  return entries.filter((entry) => entry.name === name).map((entry) => entry.duration);
}
