import type { Bar, Issue, Milestone, Project, ProjectStatus, Status, StatusCategory } from './bindings';

// Pixels per day. shortcut: fixed widths, upgrade to fit-to-width when a view is rarely wider than the screen.
export const ZOOMS = { Day: 40, Week: 14, Month: 4, Quarter: 1.5 } as const;
export type Zoom = keyof typeof ZOOMS;

export const addDays = (date: string, n: number) => {
  const d = new Date(`${date}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + n);
  return d.toISOString().slice(0, 10);
};
export const dayDiff = (a: string, b: string) => Math.round((Date.parse(`${b}T00:00:00Z`) - Date.parse(`${a}T00:00:00Z`)) / 86_400_000);

export type Span = { start: string; end: string };

/** Min start and max end of the spans; null when there are none. ISO dates compare as strings. */
export function rollup(spans: Span[]): Span | null {
  if (!spans.length) return null;
  return {
    start: spans.reduce((m, s) => (s.start < m ? s.start : m), spans[0].start),
    end: spans.reduce((m, s) => (s.end > m ? s.end : m), spans[0].end),
  };
}

export type Row = {
  id: string;
  kind: 'bar' | 'rollup' | 'milestone' | 'group';
  label: string;
  key?: string;
  title?: string;
  indent: number;
  start?: string;
  end?: string;
  issueId?: number;
  projectId?: number;
  critical?: boolean;
  category?: StatusCategory;
  icon?: string | null;
  color?: string | null;
  estimated?: boolean;
};

type Input = {
  mode: 'issues' | 'projects';
  issues: Issue[];
  bars: Bar[];
  projects: Project[];
  milestones: Milestone[];
  critical: number[];
  statuses?: Status[];
  projectStatuses?: ProjectStatus[];
};

const milestoneRows = (ms: Milestone[], indent: number): Row[] =>
  ms.filter((m) => m.target_date).map((m) => ({
    id: `milestone-${m.id}`, kind: 'milestone', label: m.name, indent, start: m.target_date!, end: m.target_date!,
  }));

/** Rows top to bottom. Issues mode groups by project when the view spans several; parents with dated children get a roll-up bar. */
export function buildRows({ mode, issues, bars, projects, milestones, critical, statuses = [], projectStatuses = [] }: Input): Row[] {
  if (mode === 'projects') {
    // A dateless project has no row, so its milestones would hang without one: they are left out with it.
    return projects.filter((p) => p.start_date && p.target_date).flatMap((p): Row[] => [
      {
        id: `project-${p.id}`, kind: 'bar', label: p.name, indent: 0, start: p.start_date!, end: p.target_date!, projectId: p.id,
        color: projectStatuses.find((s) => s.id === p.status_id)?.color ?? null,
      },
      ...milestoneRows(milestones.filter((m) => m.project_id === p.id), 1),
    ]);
  }
  const barOf = new Map(bars.map((b) => [b.issue_id, b]));
  const inView = new Set(issues.map((i) => i.id));
  const childrenOf = (id: number) => issues.filter((i) => i.parent_id === id);
  // Dated descendants of an issue, itself included when it has a bar.
  const spans = (i: Issue): Span[] => [...(barOf.has(i.id) ? [barOf.get(i.id)!] : []), ...childrenOf(i.id).flatMap(spans)];
  const issueRows = (i: Issue, indent: number): Row[] => {
    const kids = childrenOf(i.id);
    const kidSpan = rollup(kids.flatMap(spans));
    const own = barOf.get(i.id);
    const span = kidSpan ?? own;
    const st = statuses.find((s) => s.id === i.status_id);
    const row: Row[] = span
      ? [{
          id: `issue-${i.id}`, kind: kidSpan ? 'rollup' : 'bar', label: `${i.key} ${i.title}`, key: i.key, title: i.title, indent, issueId: i.id,
          start: span.start, end: span.end, critical: critical.includes(i.id), estimated: !kidSpan && own?.from_estimate,
          category: st?.category, icon: st?.icon, color: st?.color,
        }]
      : [];
    return [...row, ...kids.flatMap((k) => issueRows(k, indent + 1))];
  };
  const top = issues.filter((i) => i.parent_id === null || !inView.has(i.parent_id));
  const ids = [...new Set(top.map((i) => i.project_id))];
  if (ids.length <= 1) return [...top.flatMap((i) => issueRows(i, 0)), ...milestoneRows(milestones, 0)];
  return ids.flatMap((pid): Row[] => {
    const inProject = top.filter((i) => i.project_id === pid);
    const total = rollup(inProject.flatMap(spans));
    // A project with nothing scheduled would be an empty band; its issues are listed as unscheduled.
    if (!total) return [];
    return [
      {
        id: `group-${pid}`, kind: 'group', label: projects.find((p) => p.id === pid)?.name ?? 'No project', indent: 0,
        projectId: pid ?? undefined, ...total,
      },
      ...inProject.flatMap((i) => issueRows(i, 1)),
      ...milestoneRows(milestones.filter((m) => m.project_id === pid), 1),
    ];
  });
}

/** Issues that got no row (no dates, no dated children), listed under the Unscheduled band. */
export const unscheduled = (issues: Issue[], rows: Row[]) => issues.filter((i) => !rows.some((r) => r.issueId === i.id));

/** First and last day to draw, padded; always includes `today` so the marker is on the chart. */
export function range(rows: Row[], today: string): Span {
  const r = rollup([...(rows.filter((x) => x.start && x.end) as Span[]), { start: today, end: today }])!;
  return { start: addDays(r.start, -3), end: addDays(r.end, 14) };
}

// Zone offset in ms at an instant: the zone's wall clock read as UTC, minus the instant.
const offsetAt = (tz: string, t: number) => {
  const p = Object.fromEntries(
    new Intl.DateTimeFormat('en-US', { timeZone: tz, hourCycle: 'h23', year: 'numeric', month: 'numeric', day: 'numeric', hour: 'numeric', minute: 'numeric', second: 'numeric' })
      .formatToParts(new Date(t)).map((x) => [x.type, +x.value]),
  );
  return Date.UTC(p.year, p.month - 1, p.day, p.hour, p.minute, p.second) - Math.floor(t / 1000) * 1000;
};

/** Moves a UTC instant by whole days of `tz` local time, so the wall-clock time survives a DST change. */
export function shiftLocalDays(utc: string, tz: string, days: number): string {
  const t = Date.parse(utc);
  const wall = t + offsetAt(tz, t) + days * 86_400_000;
  // The offset at the new date may differ; one re-check settles it except inside the DST gap itself.
  const guess = wall - offsetAt(tz, wall - offsetAt(tz, t));
  return new Date(wall - offsetAt(tz, guess)).toISOString().replace('.000Z', 'Z');
}

/** The calendar day of an instant in `tz` (the zone is passed in so the result never depends on the machine). */
export const localDay = (now: Date, tz: string) => new Intl.DateTimeFormat('en-CA', { timeZone: tz }).format(now);

/** New span after dragging the whole bar or one edge by `d` days; an edge stops at the other edge. */
export function dragDates(s: Span, mode: 'move' | 'start' | 'end', d: number): Span {
  if (mode === 'move') return { start: addDays(s.start, d), end: addDays(s.end, d) };
  if (mode === 'start') {
    const start = addDays(s.start, d);
    return { start: start > s.end ? s.end : start, end: s.end };
  }
  const end = addDays(s.end, d);
  return { start: s.start, end: end < s.start ? s.start : end };
}

const ORDER: Zoom[] = ['Day', 'Week', 'Month', 'Quarter'];
/** One zoom step: 1 is closer (Day), -1 is further out (Quarter); clamps at the ends. */
export const zoomBy = (z: Zoom, dir: 1 | -1): Zoom => ORDER[Math.min(3, Math.max(0, ORDER.indexOf(z) - dir))];

/** Day offsets from `start` of every Saturday in the next `days` days; a weekend band is two days from there. */
export const saturdays = (start: string, days: number) =>
  Array.from({ length: days }, (_, k) => k).filter((k) => new Date(`${addDays(start, k)}T00:00:00Z`).getUTCDay() === 6);

const fmt = (o: Intl.DateTimeFormatOptions) => (date: string) => new Intl.DateTimeFormat('en-US', { timeZone: 'UTC', ...o }).format(new Date(`${date}T00:00:00Z`));
const weekFmt = fmt({ month: 'short', day: 'numeric' });
const monthShort = fmt({ month: 'short' });
const monthLong = fmt({ month: 'long', year: 'numeric' });

export type Tick = { k: number; label: string; sub?: string; weekend?: boolean };
export type Band = { k: number; len: number; label: string };

/**
 * The axis as two rows of whole calendar units, `k` in days from `start`. Ticks: every day (Day), every Monday (Week),
 * every 1st (Month, Quarter). Bands above them: months (Day, Week) or years (Month, Quarter). One tick per unit, so labels
 * cannot collide as long as a unit is wider than its label: a day is 40px, a week 98, a month 45 or more.
 */
export function axis(start: string, days: number, z: Zoom): { ticks: Tick[]; bands: Band[] } {
  const ticks: Tick[] = [];
  const bands: Band[] = [];
  const byMonth = z === 'Day' || z === 'Week';
  for (let k = 0; k < days; k++) {
    const d = addDays(start, k);
    const dow = new Date(`${d}T00:00:00Z`).getUTCDay();
    const dom = +d.slice(8);
    if (z === 'Day') ticks.push({ k, label: String(dom), sub: fmt({ weekday: 'short' })(d), weekend: dow === 0 || dow === 6 });
    else if (z === 'Week' && dow === 1) ticks.push({ k, label: weekFmt(d) });
    else if ((z === 'Month' || z === 'Quarter') && dom === 1) ticks.push({ k, label: monthShort(d) });
    const first = byMonth ? dom === 1 : d.slice(5) === '01-01';
    if (k === 0 || first) bands.push({ k, len: 0, label: byMonth ? monthLong(d) : d.slice(0, 4) });
  }
  bands.forEach((b, i) => (b.len = (bands[i + 1]?.k ?? days) - b.k));
  return { ticks, bands };
}
