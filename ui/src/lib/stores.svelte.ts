import { api } from './api';
import type { Issue, IssueId, Placement, Project, ProjectStatus, PropertyDef, SavedView, Status, ViewFolder, ViewId } from './bindings';

// Runes cannot be reassigned across modules, so lists are replaced in place and scalars live in a box.
export const issues = $state<Issue[]>([]);
export const statuses = $state<Status[]>([]);
export const projects = $state<Project[]>([]);
export const projectStatuses = $state<ProjectStatus[]>([]);
export const propertyDefs = $state<PropertyDef[]>([]);
export const views = $state<SavedView[]>([]);
export const folders = $state<ViewFolder[]>([]);
export const placements = $state<Placement[]>([]);
export const currentViewId = $state<{ value: ViewId | null }>({ value: null });
export const selectedIssueId = $state<{ value: IssueId | null }>({ value: null });

const replace = <T>(list: T[], next: T[]) => void list.splice(0, list.length, ...next);

/** Reloads every store from the backend; the current view's issues, or all issues when none is chosen. */
export async function refreshAll(): Promise<void> {
  const savedViews = await api.views.list();
  const current = savedViews.find((v) => v.id === currentViewId.value);
  const [i, st, p, d, f, pl, ps] = await Promise.all([
    api.views.query(
      current?.def ?? { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false },
    ),
    api.statuses.list(),
    api.projects.list(),
    api.properties.listDefs(),
    api.folders.list(),
    api.folders.items(),
    api.project.statusesList(),
  ]);
  replace(issues, i);
  replace(statuses, st);
  replace(projects, p);
  replace(propertyDefs, d);
  replace(projectStatuses, ps);
  replace(views, savedViews);
  replace(folders, f);
  replace(placements, pl);
}

/** In the desktop app the backend says when another process wrote (`db-changed`) or a link was opened. */
export async function listenToBackend(): Promise<void> {
  const { listen } = await import('@tauri-apps/api/event');
  await listen('db-changed', () => void refreshAll());
  await listen<string>('open-issue', ({ payload }) => {
    history.pushState(null, '', `/issue/${encodeURIComponent(payload)}`);
    dispatchEvent(new PopStateEvent('popstate'));
  });
}

if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) void listenToBackend();
