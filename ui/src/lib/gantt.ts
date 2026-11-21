import type { Bar, Issue, Milestone, Project } from './bindings';

// Pixels per day. shortcut: fixed widths, upgrade to fit-to-width when a view is rarely wider than the screen.
export const ZOOMS = { Day: 40, Week: 14, Month: 4, Quarter: 1.5 } as const;
export type Zoom = keyof typeof ZOOMS;
/** Days between axis ticks. shortcut: month and quarter are 30 and 90 days, not calendar boundaries. */
export const TICK_DAYS: Record<Zoom, number> = { Day: 1, Week: 7, Month: 30, Quarter: 90 };

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
  indent: number;
  start?: string;
  end?: string;
  issueId?: number;
  projectId?: number;
  critical?: boolean;
  estimated?: boolean;
};

type Input = {
  mode: 'issues' | 'projects';
  issues: Issue[];
  bars: Bar[];
  projects: Project[];
  milestones: Milestone[];
  critical: number[];
};

const milestoneRows = (ms: Milestone[], indent: number): Row[] =>
  ms.filter((m) => m.target_date).map((m) => ({
    id: `milestone-${m.id}`, kind: 'milestone', label: m.name, indent, start: m.target_date!, end: m.target_date!,
  }));

/** Rows top to bottom. Issues mode groups by project when the view spans several; parents with dated children get a roll-up bar. */
export function buildRows({ mode, issues, bars, projects, milestones, critical }: Input): Row[] {
  if (mode === 'projects') {
    return projects.flatMap((p): Row[] => [
      ...(p.start_date && p.target_date
        ? [{ id: `project-${p.id}`, kind: 'bar' as const, label: p.name, indent: 0, start: p.start_date, end: p.target_date, projectId: p.id }]
        : []),
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
    const row: Row[] = span
      ? [{
          id: `issue-${i.id}`, kind: kidSpan ? 'rollup' : 'bar', label: `${i.key} ${i.title}`, indent, issueId: i.id,
          start: span.start, end: span.end, critical: critical.includes(i.id), estimated: !kidSpan && own?.from_estimate,
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

/** Issues with an estimate and no dates, for the side lane. */
export const unscheduled = (issues: Issue[]) => issues.filter((i) => i.estimate_minutes && !i.start_date && !i.due);

/** First and last day to draw, padded; `today` when nothing is dated. */
export function range(rows: Row[], today: string): Span {
  const r = rollup(rows.filter((x) => x.start && x.end) as Span[]) ?? { start: today, end: today };
  return { start: addDays(r.start, -3), end: addDays(r.end, 14) };
}
