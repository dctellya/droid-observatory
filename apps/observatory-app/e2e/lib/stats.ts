export interface Summary {
  samples: number;
  p50: number;
  p95: number;
  max: number;
}

/** Nearest-rank percentile over the samples. */
export function summarize(values: number[]): Summary {
  const sorted = [...values].sort((a, b) => a - b);
  const rank = (p: number) => sorted[Math.max(0, Math.ceil((p / 100) * sorted.length) - 1)];
  return { samples: sorted.length, p50: rank(50), p95: rank(95), max: sorted[sorted.length - 1] };
}

export function describe(name: string, summary: Summary): string {
  const fixed = (n: number) => n.toFixed(1);
  return `${name}: samples=${summary.samples} p50=${fixed(summary.p50)}ms p95=${fixed(summary.p95)}ms max=${fixed(summary.max)}ms`;
}
