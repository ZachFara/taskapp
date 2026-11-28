// Port of crates/core/src/goals.rs, so the mock keeps goals exactly as the backend does.
import type { Api } from './api';
import type { Goal } from './bindings';

type Fail = (kind: string, message: string) => never;

export function goalsApi(rows: Goal[], projectExists: (id: number) => void, progressOf: (project: number) => [number, number], fail: Fail): Api['goals'] {
  const get = (id: number) => rows.find((g) => g.id === id) ?? fail('NotFound', `goal ${id}`);
  const check = (title: string, quarter: string) => {
    if (!title.trim()) fail('Invalid', 'a goal needs a title');
    if (!/^\d{4}-Q[1-4]$/.test(quarter)) fail('Invalid', 'quarter looks like 2027-Q1');
  };
  return {
    list: async () => [...rows].sort((a, b) => a.quarter.localeCompare(b.quarter) || a.id - b.id).map((g) => ({ ...g })),
    create: async (title, quarter, description) => {
      check(title, quarter);
      const g: Goal = { id: Math.max(0, ...rows.map((r) => r.id)) + 1, title: title.trim(), quarter, description: description ?? '', status: 'active', created_at: new Date().toISOString(), project_ids: [] };
      rows.push(g);
      return { ...g };
    },
    update: async (goal) => {
      check(goal.title, goal.quarter);
      const g = get(goal.id);
      Object.assign(g, { title: goal.title.trim(), quarter: goal.quarter, description: goal.description, status: goal.status });
      return { ...g };
    },
    delete: async (id) => {
      get(id);
      rows.splice(rows.findIndex((g) => g.id === id), 1);
      return null;
    },
    progress: async (id) => get(id).project_ids.map(progressOf).reduce<[number, number]>((a, [d, t]) => [a[0] + d, a[1] + t], [0, 0]),
    link: async (goal, project) => {
      const g = get(goal);
      projectExists(project);
      if (!g.project_ids.includes(project)) g.project_ids = [...g.project_ids, project].sort((a, b) => a - b);
      return null;
    },
    unlink: async (goal, project) => {
      const g = get(goal);
      g.project_ids = g.project_ids.filter((x) => x !== project);
      return null;
    },
  };
}

/** What the dev harness (`dev:mock`) starts with; tests and the contract cases start empty. */
export const demoGoals = (projectIds: number[]): Goal[] => [
  { id: 1, title: 'Launch the website', quarter: '2027-Q1', description: 'Public site live with docs.', status: 'active', created_at: '2026-11-20T09:00:00Z', project_ids: projectIds.slice(0, 2) },
  { id: 2, title: 'Cut support load in half', quarter: '2027-Q1', description: '', status: 'active', created_at: '2026-11-21T09:00:00Z', project_ids: projectIds.slice(2, 3) },
  { id: 3, title: 'Ship the mobile beta', quarter: '2027-Q2', description: '', status: 'active', created_at: '2026-11-22T09:00:00Z', project_ids: [] },
  { id: 4, title: 'Move to the new billing provider', quarter: '2026-Q4', description: '', status: 'achieved', created_at: '2026-10-02T09:00:00Z', project_ids: projectIds.slice(0, 1) },
];
