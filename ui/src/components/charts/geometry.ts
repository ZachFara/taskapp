// Pure chart maths: scales, paths, tick and label choices, and the text summary. No DOM, so it is tested directly.
export type Day = { date: string; scope: number; started: number; completed: number };
export type Box = { w: number; h: number; left: number; right: number; top: number; bottom: number };
type Pt = [number, number];

const r = (n: number) => String(Math.round(n * 10) / 10);

/** Ticks from 0 in a 1/2/5 step; the top tick is always above the maximum so a line never touches the edge. */
export function niceTicks(max: number, count = 4): number[] {
  if (!(max > 0)) return [0, 1];
  const raw = max / count;
  const exp = 10 ** Math.floor(Math.log10(raw));
  const step = ([1, 2, 5, 10].find((f) => f * exp >= raw - 1e-9)! * exp);
  const top = (Math.floor(max / step + 1e-9) + 1) * step;
  return Array.from({ length: Math.round(top / step) + 1 }, (_, i) => Math.round(i * step * 1e6) / 1e6);
}

/** Day index to x, value to y, inside the box's padding. `top` is the value at the top of the plot. */
export function scales(n: number, top: number, b: Box) {
  const width = b.w - b.left - b.right;
  const height = b.h - b.top - b.bottom;
  return {
    x: (i: number) => b.left + (n <= 1 ? 0 : (i / (n - 1)) * width),
    y: (v: number) => b.top + height - (v / top) * height,
  };
}

export const line = (p: Pt[]) => p.map(([x, y], i) => `${i ? 'L' : 'M'}${r(x)} ${r(y)}`).join('');
/** A value holds until the next day, then jumps: totals that change on a day are steps, not slopes. */
export const step = (p: Pt[]) => p.map(([x, y], i) => (i ? `H${r(x)}V${r(y)}` : `M${r(x)} ${r(y)}`)).join('');
export const area = (p: Pt[], base: number) =>
  p.length ? `${line(p)}L${r(p.at(-1)![0])} ${r(base)}L${r(p[0][0])} ${r(base)}Z` : '';

export const todayIndex = (days: Day[], today?: string) => {
  const i = today ? days.findIndex((d) => d.date === today) : -1;
  return i < 0 ? null : i;
};

/** Indices of the x labels: 3 to 6 on one step that lands on the last day, else a step that leaves the end bare. */
export function xLabels(n: number): number[] {
  if (n <= 1) return [0];
  const last = n - 1;
  for (let s = 1; s <= last; s++) {
    if (last % s === 0 && last / s + 1 >= 3 && last / s + 1 <= 6) return Array.from({ length: last / s + 1 }, (_, i) => i * s);
  }
  const s = Math.ceil(last / 4);
  return Array.from({ length: Math.floor(last / s) + 1 }, (_, i) => i * s);
}

export function summary(days: Day[], measure: string): string {
  const end = days.at(-1);
  if (!end) return 'No progress data yet.';
  const pct = end.scope ? Math.round((end.completed / end.scope) * 100) : 0;
  return `Progress in ${measure} from ${days[0].date} to ${end.date}. On the last day: scope ${end.scope}, started ${end.started}, completed ${end.completed} (${pct}%).`;
}

/** The strip between two lines, for the stacked variant. */
export const band = (upper: Pt[], lower: Pt[]) =>
  upper.length ? `${line(upper)}${[...lower].reverse().map(([x, y]) => `L${r(x)} ${r(y)}`).join('')}Z` : '';

/** Pushes label y positions apart so none is closer than `gap`, keeping their order. */
export function spread(ys: number[], gap: number): number[] {
  const order = ys.map((y, i) => [y, i] as const).sort((a, b) => a[0] - b[0]);
  const out = [...ys];
  let prev = -Infinity;
  for (const [y, i] of order) prev = out[i] = Math.max(y, prev + gap);
  return out;
}
